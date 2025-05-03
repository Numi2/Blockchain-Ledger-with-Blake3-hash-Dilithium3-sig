# Security Policy

## World Ledger Security Commitments

World Ledger is designed with security as a top priority. Our commitments include:

1. **Minimal, Audited Codebase:** We maintain a small, focused codebase with clear separation of concerns.
2. **Post-Quantum Security:** Using Dilithium signatures to ensure long-term cryptographic security.
3. **Memory Safety:** Leveraging Rust's memory safety guarantees and additional protections like `zeroize`.
4. **Conservative Cryptography:** Using well-vetted cryptographic primitives (BLAKE3, AES-GCM, Argon2id).
5. **Secure by Default:** Security features are enabled by default with safe configurations.
6. **Transparency:** All security enhancements and fixes are publicly documented.

## Supported Versions

Only the latest major version will receive security updates. We strongly encourage using the most recent release.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Vulnerability Disclosure Process

We appreciate the work of security researchers in improving the security of World Ledger. We are committed to working with the community to verify, address, and publish information about security vulnerabilities.

### Reporting a Vulnerability

**Do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to [security@worldledger.org](mailto:security@worldledger.org). If possible, encrypt your message with our PGP key (available at [https://worldledger.org/pgp-key.txt](https://worldledger.org/pgp-key.txt)).

Please include the following information:

1. Description of the vulnerability
2. Steps to reproduce the issue
3. Potential impact of the vulnerability
4. Any potential mitigations you've identified
5. Whether you want to be credited for the finding

### Processing Timeline

- **Acknowledgment:** We aim to acknowledge receipt of vulnerability reports within 24 hours.
- **Status Updates:** We will provide an initial assessment within 7 days.
- **Patch Development:** We aim to release patches for critical vulnerabilities within 30 days.
- **Public Disclosure:** Vulnerabilities will be disclosed publicly 90 days after the fix is released, or as coordinated with the reporter.

### Disclosure Policy

- We practice responsible disclosure.
- We will coordinate with reporters on the disclosure timeline.
- We will credit researchers who report vulnerabilities unless they prefer to remain anonymous.

### Bug Bounty Program

We do not currently offer a formal bug bounty program, but we are grateful for all security contributions and may offer discretionary rewards for significant findings.

## Security Requirements

### Key Management

- All private keys must be stored encrypted at rest.
- Memory-hardened password hashing (Argon2id) must be used for all encryption.
- Validator keys should be rotated regularly as defined in the operator documentation.
- Hardware security modules (HSMs) are strongly recommended for production validators.

### Network Security

- All RPC endpoints should be protected with TLS.
- Access to administrative functions should be restricted by IP address.
- Nodes should be operated behind firewalls with only necessary ports exposed.
- Protection against DDoS attacks should be implemented at the network level.

### Operational Security

- Regular security updates should be applied to node environments.
- Access to node infrastructure should be strictly controlled and logged.
- Regular backups should be performed and tested for restoration.
- Monitoring should be implemented to detect unusual activity.

## Threat Model

World Ledger's threat model accounts for the following adversaries:

1. **Network Attackers:** May observe or manipulate unprotected network traffic.
2. **Malicious Validators:** May attempt to undermine consensus or fork the chain.
3. **Smart Contract Exploiters:** May attempt to find vulnerabilities in contract code.
4. **Client Software Exploiters:** May attempt to exploit node software.
5. **Post-Quantum Attackers:** May have access to quantum computers to break traditional cryptography.

## Security Audits

We commit to conducting regular security audits of the codebase by independent third parties. Audit results will be made public, along with our action plan for addressing any identified issues.

## Security Architecture

### Cryptographic Primitives

- **Hash Functions:** BLAKE3 (high-performance, cryptographically secure hash function)
- **Signatures:** Dilithium3 (post-quantum secure) and Ed25519 (for backward compatibility)
- **Symmetric Encryption:** AES-256-GCM (authenticated encryption)
- **Key Derivation:** Argon2id (memory-hard password hashing)

### Defense in Depth

World Ledger implements a defense-in-depth strategy:

1. **Input Validation:** All external inputs are validated at multiple levels.
2. **Rate Limiting:** API and network communications are rate-limited to prevent DoS.
3. **Resource Constraints:** All operations have appropriate resource limits.
4. **Sandboxing:** Contract execution is isolated from node operations.
5. **Defensive Programming:** Fail-safe defaults and error handling.

## Incident Response

In the event of a security incident, we will:

1. **Assess the Impact:** Determine the scope and severity of the issue.
2. **Contain the Threat:** Take immediate actions to prevent further damage.
3. **Develop & Test Fixes:** Create and validate patches.
4. **Deploy Fixes:** Roll out updates to affected components.
5. **Disclose Responsibly:** Inform stakeholders with appropriate timing.
6. **Post-Incident Analysis:** Conduct a review to prevent similar issues.

## Additional Guidance

### For Node Operators

- Follow the security guidelines in the [Node Operator Guide](./operators/node_guide.md).
- Implement all recommended security controls for your environment.
- Keep your node software updated with the latest security patches.

### For Developers

- Follow the secure development guidelines in the [Developer Documentation](./developers/index.md).
- Use the provided security libraries for cryptographic operations.
- All code should be reviewed for security vulnerabilities before merging.

## Security Contacts

For security-related inquiries, contact:

- Security Team: [security@worldledger.org](mailto:security@worldledger.org)
- PGP Key: [https://worldledger.org/pgp-key.txt](https://worldledger.org/pgp-key.txt)

For non-security issues, please use GitHub issues or our community forums. 