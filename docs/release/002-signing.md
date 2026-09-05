# Production Code Signing

Production tags are intentionally blocked until platform signing credentials are configured.
Pull-request installer builds remain unsigned and still run package-content and launch smoke
tests.

Never commit certificates, passwords, client secrets, or Apple credentials. Store them as
GitHub repository or environment secrets.

## Tauri updater

Every Windows, macOS, and Linux update must carry a Tauri updater signature in addition to
the operating-system package signature. Generate the updater key pair once on a trusted
maintainer machine from the repository root:

```bash
pnpm tauri signer generate -w ~/.tauri/auralis.key
```

Use a strong, non-empty password. Then configure GitHub:

- secret `TAURI_SIGNING_PRIVATE_KEY`: contents of the generated private key file;
- secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the key password;
- repository variable `AURALIS_UPDATER_PUBLIC_KEY`: contents printed for the public key.

Keep an encrypted offline backup of the private key and its password. The public key is
embedded into production builds and is safe to distribute, but the private key must never be
committed or shared. Losing it means existing installations cannot trust any future update;
replacing it requires distributing a new installer through another trusted channel.

Release CI refuses empty or placeholder updater credentials. Local development builds do not
embed the production public key and therefore cannot contact or install production updates.

## Windows

Set the repository variable `WINDOWS_SIGNING_MODE` to one of the supported modes.

### PFX mode

Configure these secrets:

- `WINDOWS_CERTIFICATE`: base64-encoded code-signing PFX;
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX export password.

The optional repository variable `WINDOWS_TIMESTAMP_URL` overrides the default DigiCert
RFC 3161 timestamp service.

### Azure Artifact Signing mode

Set `WINDOWS_SIGNING_MODE=azure` and configure:

- secrets: `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`;
- repository variables: `AZURE_ARTIFACT_SIGNING_ENDPOINT`,
  `AZURE_ARTIFACT_SIGNING_ACCOUNT`, `AZURE_ARTIFACT_SIGNING_PROFILE`, and
  `AZURE_ARTIFACT_SIGNING_CLI_VERSION`.

The CLI version is mandatory so a release never installs an unpinned signing tool.

## macOS

Configure these secrets for Developer ID signing and notarization:

- `APPLE_CERTIFICATE`: base64-encoded Developer ID Application `.p12`;
- `APPLE_CERTIFICATE_PASSWORD`: `.p12` export password;
- `APPLE_ID`: Apple account email;
- `APPLE_PASSWORD`: app-specific Apple account password;
- `APPLE_TEAM_ID`: Apple Developer team identifier.

Tauri imports the certificate, infers the signing identity, submits the bundle for
notarization, and staples the result. The release gate verifies `codesign`, Gatekeeper
assessment, and the stapled DMG ticket.

## Release enforcement

For every production tag, the workflow performs these gates in order:

1. validate that the target platform has complete signing configuration;
2. build and sign the native packages;
3. verify bundled media tools and compliance files;
4. verify Authenticode or Apple signing and notarization;
5. install and launch the Windows or macOS package.

Any failed platform gate prevents the publish job from running, so no GitHub Release is
created from unverified packages. After every platform succeeds, the only write-enabled job
downloads the verified workflow artifacts, validates the complete package set, adds
the signed updater assets, `latest.json`, and `SHA256SUMS.txt`, and creates one draft release.
