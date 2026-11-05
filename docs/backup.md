# Local backup and portable definitions

In Settings → Backup, choose **Create encrypted backup**, enter and confirm a
passphrase of at least 12 characters, read the data disclosure, and select a new
output filename. Keep the passphrase separately: there is no recovery service.
Native file selection currently uses the Linux desktop file chooser portal; an
unavailable portal reports failure. Other platforms report unsupported selection.
Existing files are refused even if the chooser offers to replace them.

## Included data

Version 1 contains a consistent committed SQLCipher/SQLite WAL snapshot of cached
message bodies and headers, attachment metadata, accounts, folders/checkpoints and
memberships, read/star/snooze state, tags, local folder overrides, photos, category
metadata/corrections, sender-review choices, due-date dismissals, rule definitions,
saved views, disabled alert destination metadata, reminder definitions, alert
cadence/quiet-hour definitions and the FTS tokenizer preference. Message contents
and user-authored definitions can contain sensitive information.

Unfetched attachment binaries are absent. External downloaded attachment files,
model files, browser preferences (including language), vault contents, database
keys, credentials, IMAP connection configurations, AI configurations/permissions,
secret integration URLs and authentication are excluded. Opaque vault references,
public destination origins, urgent bypass permissions, delivery history, pending
remote mutations, rule/reminder/alert/digest jobs, preview tokens and live leases
are excluded. Search vectors and conversation indexes are rebuilt locally as
applicable; semantic search requires separate model/permission setup.

## Restore and reconnect

Restore is offered before initial setup **only when the application's data
directory is absent**. Keep an existing installation intact; this is fresh-target
recovery, not merge or in-place replacement. It does not replace an existing
OS-keyring entry. Select the backup, confirm its passphrase and the disclosure,
then continue and unlock the recovered local vault with the backup passphrase.
Recovery intentionally uses an isolated encrypted file vault rather than the
OS keyring, even when a keyring is available.

Accounts appear as cached-only and cannot start network workers. In Settings →
Accounts, add the **same exact email address** and configure authentication/host
again to reconnect while retaining the restored account ID and cached mail.
Sender review/hiding, rules, reminders and alert destinations remain disabled;
AI/network consent is reset. Inspect and explicitly reconfigure/enable desired
features. Old work queues are never replayed. Restoring does not fetch attachments
or contact any provider. Subsequent explicitly reconnected sync follows normal
sync and new-mail eligibility rules.

Recovery builds and validates a new encrypted database and vault in a private
sibling staging directory, then publishes the complete directory. Wrong passwords,
unsupported formats, malformed records and failed integrity checks leave the
original installation intact. A crash before publication can leave a private
`.triage-recovery-*` sibling directory; it is never auto-opened and retrying creates
a different stage. After closing the app, an abandoned stage can be removed.
A crash after publication leaves the complete recovered installation available
for unlock. Storage hardware/filesystem durability guarantees still apply.

## Portable rules and saved views

**Export rules & saved views** writes readable version-1 JSON. Names, search text
and predicates can contain private text, so inspect before sharing. The exporter
replaces account/destination IDs with placeholders and exports no credentials or
integration configuration. All rules in the file are disabled.

**Choose definitions to import** previews the full document. Select every account
and destination mapping, inspect mailbox scopes, then confirm. Missing targets,
unknown versions, invalid scopes and capacity overflow fail without a partial
import. All-account definitions intentionally retain all-account scope. Imports
append definitions with new IDs/revisions, keep rules disabled and never run rules
against historical mail. Existing definitions remain intact. The file is re-read
and compared with the preview at commit; any change requires another preview.

## Format and bounds

The encrypted stream starts with `TMBACK01` and a random 16-byte salt. Version 1
fixes Argon2id v19 at 19 MiB, two passes and one lane, deriving an ephemeral
AES-256-GCM key. Each record has a fresh random 96-bit nonce, authenticated sequence
number and record/final marker. A mandatory authenticated final record and EOF
check reject truncation, duplicated/reordered records and trailing bytes. Format
and schema versions are encrypted in the manifest. The original database key is
never included. No plaintext database staging file is created.

Version 1 accepts schema 10 only; unknown format/schema versions fail closed.
Future compatibility requires an explicit reader/migration, not guessing. Records
are bounded to 64 MiB, restore to ten million rows and a 64 GiB input file; portable
JSON is bounded to 4 MiB and existing rule/view capacities. No compression or
archive paths are accepted. Native paths must be absolute and have canonical
parents; final symlinks, parent traversal and occupied outputs are refused.

Backup holds the main storage connection for the duration of one read transaction;
large snapshots can delay ingestion and UI queries. Memory is bounded by a record,
not by total cached-mail size. No native responsiveness/100k-message throughput
claim or live desktop-portal interoperability claim is established by unit tests.
