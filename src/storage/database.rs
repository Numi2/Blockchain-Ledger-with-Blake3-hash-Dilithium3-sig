use crate::storage::{Column, ColumnFamilyHandles, StorageError, StorageResult};
use rocksdb::{Options, DB, WriteBatch, IteratorMode, ReadOptions, Direction};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Database wrapper for RocksDB
pub struct Database {
    /// The RocksDB instance
    db: Arc<DB>,
    
    /// Column family handles
    cf_handles: ColumnFamilyHandles,
    
    /// Path to the database
    path: PathBuf,
}

impl Database {
    /// Open a database at the given path, creating it if it doesn't exist
    pub fn open<P: AsRef<Path>>(path: P) -> StorageResult<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        
        // Configure database options
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);
        options.set_max_background_jobs(4);
        options.set_max_total_wal_size(512 * 1024 * 1024); // 512MB
        options.set_keep_log_file_num(10);
        options.set_write_buffer_size(256 * 1024 * 1024); // 256MB
        options.set_max_write_buffer_number(6);
        options.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        options.set_level_zero_file_num_compaction_trigger(4);
        options.set_level_zero_slowdown_writes_trigger(8);
        options.set_level_zero_stop_writes_trigger(12);
        
        // Open database with column families
        let descriptors = Column::descriptors();
        let db = DB::open_cf_descriptors(&options, &path, descriptors)?;
        let db = Arc::new(db);
        
        // Get column family handles
        let mut handles = Vec::with_capacity(Column::all().len());
        for column in Column::all() {
            let handle = db.cf_handle(column.name())
                .ok_or_else(|| StorageError::Other(format!("Column family not found: {}", column.name())))?;
            handles.push(Arc::new(handle));
        }
        
        Ok(Self {
            db,
            cf_handles: ColumnFamilyHandles::new(handles),
            path,
        })
    }
    
    /// Get database path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Get the value for the given key from the specified column
    pub fn get(&self, column: Column, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let cf = self.cf_handles.get(column);
        Ok(self.db.get_cf(&cf, key)?)
    }
    
    /// Put a key-value pair into the specified column
    pub fn put(&self, column: Column, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let cf = self.cf_handles.get(column);
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }
    
    /// Delete a key from the specified column
    pub fn delete(&self, column: Column, key: &[u8]) -> StorageResult<()> {
        let cf = self.cf_handles.get(column);
        self.db.delete_cf(&cf, key)?;
        Ok(())
    }
    
    /// Create a write batch for atomic operations
    pub fn batch(&self) -> WriteBatchWrapper {
        WriteBatchWrapper {
            batch: WriteBatch::default(),
            db: Arc::clone(&self.db),
            cf_handles: self.cf_handles.clone(),
        }
    }
    
    /// Check if a key exists in the specified column
    pub fn exists(&self, column: Column, key: &[u8]) -> StorageResult<bool> {
        self.get(column, key).map(|opt| opt.is_some())
    }
    
    /// Iterate over all key-value pairs in a column
    pub fn iter(&self, column: Column) -> StorageResult<DatabaseIterator> {
        let cf = self.cf_handles.get(column);
        let iter = self.db.iterator_cf(&cf, IteratorMode::Start)?;
        Ok(DatabaseIterator { iter })
    }
    
    /// Iterate over a range of key-value pairs in a column
    pub fn range_iter(&self, column: Column, from_key: &[u8], to_key: &[u8]) -> StorageResult<DatabaseIterator> {
        let cf = self.cf_handles.get(column);
        let mut read_opts = ReadOptions::default();
        
        // Set range for iteration
        read_opts.set_iterate_upper_bound(to_key.to_vec());
        let iter = self.db.iterator_cf_opt(&cf, read_opts, IteratorMode::From(from_key, Direction::Forward))?;
        
        Ok(DatabaseIterator { iter })
    }
    
    /// Get RocksDB statistics
    pub fn get_statistics(&self) -> Option<String> {
        self.db.property_value("rocksdb.stats")
    }
    
    /// Compact the database to reclaim space
    pub fn compact(&self) -> StorageResult<()> {
        for column in Column::all() {
            let cf = self.cf_handles.get(*column);
            self.db.compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
        }
        Ok(())
    }
}

/// Wrapper for RocksDB WriteBatch for atomic operations
pub struct WriteBatchWrapper {
    /// The RocksDB write batch
    batch: WriteBatch,
    
    /// The database instance
    db: Arc<DB>,
    
    /// Column family handles
    cf_handles: ColumnFamilyHandles,
}

impl WriteBatchWrapper {
    /// Put a key-value pair into the specified column
    pub fn put(&mut self, column: Column, key: &[u8], value: &[u8]) -> &mut Self {
        let cf = self.cf_handles.get(column);
        self.batch.put_cf(&cf, key, value);
        self
    }
    
    /// Delete a key from the specified column
    pub fn delete(&mut self, column: Column, key: &[u8]) -> &mut Self {
        let cf = self.cf_handles.get(column);
        self.batch.delete_cf(&cf, key);
        self
    }
    
    /// Write all batched operations to the database
    pub fn write(self) -> StorageResult<()> {
        self.db.write(self.batch)?;
        Ok(())
    }
}

/// Iterator over key-value pairs in the database
pub struct DatabaseIterator<'a> {
    /// The RocksDB iterator
    iter: rocksdb::DBIterator<'a>,
}

impl<'a> Iterator for DatabaseIterator<'a> {
    type Item = (Box<[u8]>, Box<[u8]>);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
} 