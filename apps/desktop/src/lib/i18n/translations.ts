import type { SupportedLocale, Translations } from './types';

export const translations: Record<SupportedLocale, Translations> = {
	en: {
		mailboxes: {
			mailboxes: 'Mailboxes',
			navigation: 'Mail navigation',
			allMail: 'All Mail',
			Inbox: 'Inbox',
			Starred: 'Starred',
			Snoozed: 'Snoozed',
			spam: 'Spam',
			archive: 'Archive',
			trash: 'Trash',
			other: 'Other folders',
			loadingFolders: 'Loading folders…',
			catalogError: 'Folders unavailable. Cached mail remains accessible.',
			notSynced: 'Folders not yet synced.',
			retry: 'Try again',
			unavailable: 'This folder is no longer available. Choose another mailbox.',
			noCached: 'No cached messages in this view.',
			empty: 'No messages found',
			emptyHelp: 'Try another search or mailbox.',
			star: 'Star',
			unstar: 'Unstar',
			snooze: 'Snooze',
			unsnooze: 'Unsnooze',
			snoozedUntil: 'Snoozed until',
			snoozeHelp: 'Choose a local date and time within the next year. Saved on this device only.',
			deadline: 'Remind me at',
			saveSnooze: 'Save snooze',
			invalidDeadline: 'Choose a future date within one year.',
			actions: 'Message actions',
			close: 'Close',
			markRead: 'Mark read',
			markUnread: 'Mark unread',
			moveTrash: 'Move to Trash',
			tag: 'Add local tag',
			unsubscribe: 'Send one-click unsubscribe request',
			searchScope: 'Search this mailbox',
			settings: 'Settings',
			dueDates: 'Due Dates',
			noActions: 'No matching actions.',
			actionSearch: 'Find a message action',
			saved: 'Saved locally. Server changes sync in the background.',
			localSaved: 'Saved on this device.',
			saveError: 'Change could not be saved. Please try again.',
			loadError: 'Mail could not be loaded. Please try again.',
			vaultError: 'Unlock your local vault to access mail.',
			loading: 'Loading messages…'
		},
		app: {
			title: 'Triage Mail',
			tagline: 'Local storage. Private credentials. Your choice of provider.'
		},
		language: {
			en: 'EN',
			ptBR: 'PT',
			selectLabel: 'Select language'
		},
		accountPhoto: {
			label: 'Account photo',
			choose: 'Choose a photo',
			replace: 'Replace photo',
			remove: 'Remove photo',
			help: 'Local JPEG, PNG or WebP · up to 2 MiB and 1024 × 1024 pixels.',
			pending: 'Updating photo…',
			loadError: 'Photo could not be loaded. Reopen account settings to retry.',
			saveError: 'Photo could not be saved. Choose a valid image or try again.',
			invalidFile: 'Choose a nonempty image up to 2 MiB.'
		},
		onboarding: {
			unlockTitle: 'Unlock your mail',
			accountsTitle: 'Your accounts',
			openingMail: 'Opening local mail…',
			vaultLockedHelp:
				'Your system keyring is unavailable or the vault is locked. Enter your local vault passphrase. On first use, this creates an encrypted fallback vault.',
			vaultPassphraseLabel: 'Vault passphrase',
			unlocking: 'Unlocking…',
			unlockVault: 'Unlock vault',
			retryKeyring: 'Retry system keyring',
			tryAgain: 'Try opening mail again',
			connectedAccounts: 'Connected accounts',
			openInbox: 'Open inbox',
			connectFirstAccount: 'Connect your first account to start bringing your mail together.',
			accountSaveError: 'Account could not be saved',
			pageTitle: 'Vault Unlock & Multi-Account Onboarding',
			vaultLockedBadge: 'Vault Sealed',
			vaultUnlockedBadge: 'Vault Unlocked',
			recoveryModeBadge: 'Recovery Available',
			demoPreviewBadge: 'Demo Preview',
			accountsIndexed: 'accounts indexed',
			viewConnectedAccounts: 'View connected accounts',
			addAccountAction: 'Add account',
			backToInbox: 'Back to Inbox',
			continueWithoutRestore: 'Continue without restoring',
			restoreBackupAction: 'Restore from Encrypted Backup (.triagebak)',
			restoreBackupTitle: 'Restore from Encrypted Backup',
			restoreBackupHelp:
				'Choose an encrypted .triagebak archive and confirm your backup passphrase to restore local mail before initialization.',
			restorePassphraseLabel: 'Backup passphrase (min. 12 characters)',
			restoreConfirmLabel: 'Confirm backup passphrase',
			restoreConsentLabel:
				'I understand this restores encrypted mail metadata and keeps automation disabled until re-enabled.',
			restoreChooseFile: 'Choose backup & restore',
			restorePassphraseMismatch:
				'Passphrase and confirmation must match and be at least 12 characters.',
			restoreSuccess: 'Encrypted backup restored. Loading local accounts…',
			restoreError: 'Backup could not be restored. Verify the file and passphrase.',
			credentialsZeroized: 'Credentials are zeroized from DOM immediately upon submission.',
			manualNetworkOnly: 'Manual network actions only',
			showPassphrase: 'Show passphrase',
			hidePassphrase: 'Hide passphrase',
			foldersCount: 'Folders',
			folderColumn: 'Folder',
			roleColumn: 'Role',
			totalColumn: 'Total',
			unreadColumn: 'Unread',
			loadingFolders: 'Loading folder catalog…',
			noFolders: 'No folders synced yet.',
			foldersUnavailable: 'Folder catalog unavailable.',
			noAccountsTitle: 'No accounts connected yet',
			noAccountsHelp:
				'Run domain-only ISPDB discovery or connect a local Proton Bridge loopback mailbox.',
			configureFirstAccount: 'Configure First Account',
			vaultUnlockError: 'Vault could not be unlocked. Check your passphrase and try again.',
			createVaultTitle: 'Create local master passphrase',
			createVaultBadge: 'New Local Vault',
			vaultCreateHelp:
				'Set a local master passphrase to encrypt your vault and mail database on this device.',
			vaultCreateKeyringUnavailableHelp:
				'No OS keyring (Secret Service) is available in this session. Create a local master passphrase to encrypt your accounts and mail database with Argon2id + AES-GCM. Keep it safe—it cannot be recovered if lost.',
			vaultConfirmPassphraseLabel: 'Confirm master passphrase',
			vaultPassphraseMinHint: 'Use at least 8 characters.',
			vaultPassphraseMismatch:
				'Passphrase and confirmation must match and be at least 8 characters.',
			createVaultAction: 'Create vault & continue',
			creatingVault: 'Creating vault…',
			reauthorize: 'Reauthorize',
			reauthorizing: 'Reauthorizing…',
			reauthorizeHelp:
				'Sign in again in your browser to refresh OAuth tokens without deleting locally indexed mail.',
			reauthorizeClientOverrideHint:
				'Leave Client ID and Client secret blank to reuse the credentials saved in your vault.',
			reauthorizeSubmit: 'Start browser sign-in',
			reauthorizeCancel: 'Cancel',
			reauthorizeSuccess:
				'OAuth tokens refreshed. Sync resumed without touching local mail.',
			reauthorizeError:
				'Reauthorization did not complete. Review the guidance below and try again.',
			accountDiagnosticBanner: 'Connection attention required',
			planBadge: 'Plan / Badge',
			planAuto: 'Auto (IMAP Quota)',
			planHidden: 'Hidden',
			planCustom: 'Custom',
			planCustomLabel: 'Custom label (max 12 characters)',
			planApply: 'Apply',
			planError: 'The badge could not be saved. Try again.',
			storageUsage: 'Storage used'
		},
		accountForm: {
			discoveryIntro:
				'Enter your email address to find offline provider settings. You can also choose a preset or enter settings manually.',
			advancedSettings: 'Advanced connection settings',
			microsoftSetup: 'Set up Microsoft sign-in',
			microsoftSetupHelp:
				'Use your own desktop application client ID, then authorize in your browser.',
			onlineLookup: 'Look up settings online',
			onlineLookupHelp:
				'This sends only your email domain to Mozilla Thunderbird ISPDB and replaces the connection settings. It does not sign in or test a server.',
			confirmProbe: 'Confirm and test',
			probeHelp:
				'Confirm the host below to test its IMAP TLS connection on port 993. No credentials are sent.',
			discovering: 'Checking provider settings…',
			providerFound: 'Suggested settings:',
			discoveryUnresolved:
				'No matching settings found. Enter your provider’s IMAP host in advanced settings.',
			discoveryInvalid: 'Enter a single valid email address.',
			discoveryUnavailable:
				'Settings lookup is unavailable. Choose a preset or enter settings manually.',
			probeSuccess: 'IMAP TLS connection confirmed. Sign-in has not been tested.',
			probeFailed:
				'IMAP TLS could not be confirmed. Check the host and try again or configure it manually.',
			gmailAppPasswordHelp: 'Create a Google app password (requires 2-Step Verification)',
			gmailRegularPasswordHint:
				'This does not look like a Google app password (16 letters). A regular Gmail password will be rejected. Create an app password first.',

			title: 'Connect an account',
			subtitle: 'Use the connection settings supplied by your mail provider.',
			methodLabel: 'Sign-in method',
			methodAppPassword: 'App password',
			methodOAuthPublic: 'OAuth · Public PKCE',
			methodOAuthByo: 'OAuth · Your client credentials',
			methodProton: 'Proton Mail Bridge',
			methodLocalArchive: 'Offline Archive',
			emailLabel: 'Email address or IMAP username',
			emailLabelBridge: 'Bridge username',
			hostLabel: 'IMAP host',
			portLabel: 'Port',
			oauthHelp:
				'Register a desktop client with your provider using a loopback redirect at http://127.0.0.1/callback with a dynamic port supported by your provider. Authorization opens in your browser.',
			authEndpointLabel: 'Authorization endpoint (HTTPS)',
			tokenEndpointLabel: 'Token endpoint (HTTPS)',
			scopesLabel: 'IMAP scopes',
			scopesHelp:
				'Space-separated provider scopes. Include offline access if your provider supports it.',
			clientIdLabel: 'Client ID',
			clientSecretLabel: 'Client secret',
			bridgePasswordLabel: 'Bridge password',
			appPasswordLabel: 'App password',
			protonNotice:
				'Start Proton Mail Bridge first and select its SSL/TLS IMAP mode. Only loopback hosts are allowed; local self-signed TLS is accepted.',
			oauthVaultNotice:
				'Authorize contacts your configured provider endpoints. Tokens and client credentials stay in the native vault.',
			connectVaultNotice:
				'Connect saves credentials in the native vault and starts inbound mail sync.',
			connecting: 'Connecting…',
			authorizeAndConnect: 'Authorize and connect',
			connectAccount: 'Connect account',
			testLogin: 'Test login',
			testingLogin: 'Testing login…',
			testLoginSuccess:
				'Login verified. Credentials were cleared—re-enter your password to connect.',
			testLoginFailed:
				'Login test failed. Verify your host, port, and credentials.',
			diagnostics: {
				TlsHandshakeFailed: {
					title: 'TLS certificate or port mismatch',
					hint: 'Verify the IMAP host and port (993 for IMAPS). For Proton Mail Bridge, confirm Bridge is running on 127.0.0.1:1143 with SSL/TLS mode enabled.'
				},
				AuthenticationRejected: {
					title: 'Email or password rejected by provider',
					hint: 'Double-check your email address and password. For Gmail, iCloud, Yahoo, or Zoho, enable 2-Step Verification and generate a 16-character App Password instead of your account password.'
				},
				ImapDisabledHint: {
					title: 'IMAP access is disabled on this mailbox',
					hint: 'Open your webmail settings (e.g. Gmail Forwarding and POP/IMAP, Zoho Mail Accounts -> IMAP, or UOL/BOL/Terra settings) and enable IMAP access before connecting.'
				},
				OAuthAccessDenied: {
					title: 'Provider denied OAuth authorization',
					hint: 'Access was declined or blocked. If you are using a personal Google Cloud project in Testing mode, add your email under OAuth consent screen -> Test users and approve access in the browser. Testing-mode refresh tokens expire after 7 days; publish the consent screen "In production" (free for personal use) to avoid weekly re-authorization.'
				},
				OAuthInvalidClient: {
					title: 'OAuth client configuration mismatch',
					hint: 'Check your Client ID, Client secret, and endpoints. Google Testing-mode tokens also expire after 7 days; publish the consent screen "In production" to stop this. Create a "Desktop app" (or "Public client/native") client and register http://127.0.0.1/callback as the redirect URI.'
				},
				OAuthCancelled: {
					title: 'Authorization cancelled',
					hint: 'Browser sign-in was cancelled before finishing. Start authorization again when you are ready.'
				},
				NetworkTimeout: {
					title: 'Connection timed out',
					hint: 'The server did not respond in time. Check your internet connection, firewall/VPN, and that the IMAP host and port are reachable.'
				}
			},
			browserAuthPending:
				'Finish authorization in your browser. This attempt expires in five minutes.',
			cancelAuth: 'Cancel authorization',
			quickFillLabel: 'Quick fill:',
			importGoogleJson: 'Import Google Cloud JSON',
			importGoogleJsonHelp:
				'Load the client_secret_*.json file (Desktop app) directly into the form.',
			importGoogleJsonSuccess:
				'Google Cloud OAuth credentials loaded. Enter your email and authorize.',
			importGoogleJsonWebError:
				'This JSON is for a Web application. Create a "Desktop app" OAuth client ID in Google Cloud Console.',
			importGoogleJsonInvalidError:
				'Invalid Google Cloud OAuth JSON file. Select the client_secret_*.json downloaded from Credentials.',
			localArchiveNotice:
				'Import messages from a local .mbox archive (Google Takeout, Thunderbird) or a single .eml file completely offline. No password or IMAP server is required.',
			localArchiveEmailLabel: 'Archive label or email address',
			localArchiveFormatLabel: 'Archive format',
			localArchiveFormatMbox: '.mbox (multi-message mailbox)',
			localArchiveFormatEml: '.eml (single RFC 5322 message)',
			localArchiveChooseFile: 'Choose .mbox or .eml file',
			localArchiveNoFile: 'No file selected yet',
			localArchiveSubmit: 'Import local archive',
			localArchiveImporting: 'Importing archive…',
			localArchiveProgress: 'Messages imported so far:',
			localArchiveSuccess: 'Imported messages into local archive:',
			localArchiveNoticeFooter:
				'Imported messages are indexed locally in SQLCipher storage. Repeated imports using the same email append monotonically.',
			localArchiveConflictError:
				'This email belongs to an active or recovered IMAP account. Choose a distinct email address for local archives.',
			localArchiveImportError:
				'Archive could not be imported. Verify the file format (.mbox or .eml) and permissions.',
			localArchiveBadge: 'Offline Archive (Local Only)',
			allSignInMethods: 'More sign-in methods',
			fewerSignInMethods: 'Fewer sign-in methods',
			recommendedOption: 'Recommended',
			googleCloudJsonOption: 'Google Cloud OAuth JSON',
			offlineImportOption: 'Offline .mbox / .eml Import',
			customImapOption: 'Custom IMAP / Advanced',
			resolvedServerSummary: 'Server settings resolved',
			providerGuidanceGoogle:
				'Choose App Password (easiest with 2FA), import a Google Cloud OAuth JSON desktop client, or import a Google Takeout .mbox offline.',
			providerGuidanceMicrosoft:
				'Use Microsoft sign-in (Public PKCE) with your client ID, an App Password, or import an offline .mbox/.eml archive.',
			providerGuidanceAppPassword:
				'Sign in with a provider App Password, or import an offline .mbox/.eml archive.',
			providerGuidanceProton:
				'Connect to Proton Mail Bridge running locally (127.0.0.1:1143), or import an offline .mbox/.eml archive.',
			providerGuidanceCustom:
				'Configure your IMAP server host and port below, or choose an offline .mbox/.eml import.'
		},
		guide: {
			title: 'How sign-in methods work & step-by-step setup',
			toggleOpen: 'Show guide',
			toggleClose: 'Hide guide',
			requirementsLabel: 'Requirements:',
			servicesLabel: 'Supported services:',
			fieldsTitle: 'What to fill in the form below:',
			stepsTitle: 'Step-by-step: how to obtain your credentials:',
			appPassword: {
				title: 'App Password',
				summary:
					'Generate a 16-character dedicated password in your email security panel. Does not require developer setups.',
				requirements:
					'Two-Factor Authentication (2FA) must be enabled on your account. IMAP must be turned on in webmail settings.',
				services:
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Zoho, UOL, BOL, Terra, Outlook.com, and custom IMAP servers.',
				hostTip:
					'Popular hosts: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, imap.zoho.com:993, imap.uol.com.br:993',
				fields: [
					'Email address: Your full email address (e.g. reader@example.com).',
					'IMAP host & Port: Click your provider in Quick fill (or enter your server host and port 993).',
					'App password: The 16-character password generated in your provider security settings (paste without spaces).'
				],
				playbooks: [
					{
						id: 'Gmail',
						provider: 'Google / Gmail & Google Workspace',
						steps: [
							'Open your Google Account security page (myaccount.google.com/security) and enable 2-Step Verification (2FA).',
							'Go to App passwords (myaccount.google.com/apppasswords), enter the name "Triage Mail", and click Create.',
							'Copy the 16-letter password shown on screen (ignore spaces).',
							'In Gmail on the web (Settings -> See all settings -> Forwarding and POP/IMAP), verify that IMAP access is Enabled.',
							'Click "Gmail" in Quick fill, enter your email, and paste the 16-character App Password.'
						]
					},
					{
						id: 'iCloud',
						provider: 'Apple iCloud Mail',
						steps: [
							'Sign in to appleid.apple.com (Two-Factor Authentication is required).',
							'Open Sign-In and Security -> App-Specific Passwords -> Generate an app-specific password.',
							'Name it "Triage Mail" and copy the generated xxxx-xxxx-xxxx-xxxx password.',
							'Click "iCloud" in Quick fill, enter your iCloud email address, and paste the generated password.'
						]
					},
					{
						id: 'Yahoo',
						provider: 'Yahoo Mail & AOL',
						steps: [
							'Go to Account Info -> Account Security -> Generate and manage app passwords, create one for "Triage Mail", and copy it.',
							'Click "Yahoo" in Quick fill, enter your email, and paste the App Password.'
						]
					},
					{
						id: 'Fastmail',
						provider: 'Fastmail',
						steps: [
							'Go to Settings -> Privacy & Security -> Integrations -> New app password, select IMAP access, and copy the password.',
							'Click "Fastmail" in Quick fill, enter your email, and paste the App Password.'
						]
					},
					{
						id: 'Zoho',
						provider: 'Zoho Mail',
						steps: [
							'Sign in to accounts.zoho.com -> Security -> App Passwords and generate a dedicated password for "Triage Mail" (requires 2FA).',
							'In Zoho Mail Settings -> Mail Accounts -> IMAP, verify IMAP Access is enabled.',
							'Click "Zoho" in Quick fill, enter your email, and paste the generated App Password.'
						]
					},
					{
						id: 'UOL',
						provider: 'UOL Mail',
						steps: [
							'Open UOL Segurança / Minha Conta, enable IMAP access in webmail settings, and generate an App Password if 2FA is active.',
							'Click "UOL" in Quick fill (imap.uol.com.br:993), enter your full @uol.com.br email, and paste your password.'
						]
					},
					{
						id: 'BOL',
						provider: 'BOL Mail',
						steps: [
							'Verify IMAP access is enabled in your BOL webmail settings (and generate an App Password if 2FA is enabled).',
							'Click "BOL" in Quick fill (imap.bol.com.br:993), enter your full @bol.com.br email, and paste your password.'
						]
					},
					{
						id: 'Terra',
						provider: 'Terra Mail',
						steps: [
							'Verify IMAP access is enabled in Terra Mail settings and generate an App Password if two-step verification is active.',
							'Click "Terra" in Quick fill (imap.terra.com.br:993), enter your full @terra.com.br email, and paste your password.'
						]
					}
				]
			},
			oauthPublic: {
				title: 'OAuth · Public PKCE',
				summary:
					'Browser login via official provider page with local loopback callback (RFC 7636). Does not need a client secret.',
				requirements:
					'Provider must support public desktop PKCE clients and http://127.0.0.1/callback redirects.',
				services: 'Microsoft 365, Outlook.com, Hotmail, and RFC 7636 compliant identity providers.',
				redirectTip:
					'Google blocks pure public PKCE for desktop IMAP without CASA certification; for Gmail use App Password or BYO Credentials.',
				fields: [
					'Quick fill (Outlook): Automatically populates Host (outlook.office365.com), Port (993), Authorization endpoint, Token endpoint, and IMAP scopes.',
					'Client ID: The Application (client) UUID generated in your Microsoft Entra / Azure app registration.'
				],
				playbooks: [
					{
						id: 'Outlook',
						provider: 'Microsoft (Outlook.com / Hotmail / Microsoft 365)',
						steps: [
							'Open Microsoft Entra / Azure Portal (portal.azure.com -> App registrations) and click New registration.',
							'Enter a name ("Triage Mail") and select "Accounts in any organizational directory and personal Microsoft accounts".',
							'Under Redirect URI, choose platform "Public client/native (mobile & desktop)" and enter http://127.0.0.1/callback.',
							'In API permissions -> Add a permission -> Microsoft APIs, add delegated permissions IMAP.AccessAsUser.All and offline_access.',
							'On the Overview page of your registered app, copy the Application (client) ID.',
							'Click "Outlook" in Quick fill, type your email, paste the Client ID, and click Authorize and connect.'
						]
					}
				]
			},
			oauthByo: {
				title: 'OAuth · BYO Credentials',
				summary:
					'Bring your own developer client_id and client_secret for continuous OAuth token refresh.',
				requirements:
					'Create a Desktop Application in Google Cloud Console or your enterprise identity provider.',
				services:
					'Google Cloud Console (Gmail / Google Workspace) and private enterprise SSO (Keycloak, Okta).',
				gcpTip: 'Scope required: https://mail.google.com/ with loopback redirect. Google Testing-mode refresh tokens expire after 7 days: publish the consent screen "In production" (free for personal use).',
				fields: [
					'Quick fill (Gmail OAuth): Auto-fills Host (imap.gmail.com), Port (993), Google OAuth endpoints, and the https://mail.google.com/ scope.',
					'Client ID & Client secret: The Desktop OAuth client credentials created in your Google Cloud Console project.'
				],
				playbooks: [
					{
						id: 'Gmail OAuth',
						provider: 'Google Cloud Console (Gmail / Google Workspace)',
						steps: [
							'Open console.cloud.google.com, create a project, and enable the "Gmail API" under APIs & Services -> Library.',
							'Go to APIs & Services -> OAuth consent screen, choose External (or Internal for Workspace), add scope https://mail.google.com/, and add your own email under Test users.',
							'Go to Credentials -> Create Credentials -> OAuth client ID and select Application type "Desktop app".',
							'Download the client_secret_*.json file (or copy both the generated Client ID and Client secret).',
							'Click "Gmail OAuth" (or "Google Workspace") in Quick fill, click "Import Google Cloud JSON" to auto-fill your credentials (or paste them manually), enter your email, and click Authorize and connect.',
							'In the browser window, sign in and approve access (if Google shows an unverified-app prompt for your personal project, click Advanced -> Proceed).'
						]
					}
				]
			},
			proton: {
				title: 'Proton Mail Bridge',
				summary:
					'Connects to the official Proton Mail Bridge app running locally on your computer (127.0.0.1:1143) to read zero-access encrypted email.',
				requirements:
					'Active paid Proton subscription and Proton Mail Bridge app installed and running in the background.',
				services: 'Proton Mail exclusively.',
				warning:
					'IMPORTANT: Use the username and password displayed inside the Proton Mail Bridge app, NOT your master Proton account password.',
				fields: [
					'Host & Port: Fixed to 127.0.0.1 and 1143 (auto-filled when selecting Proton).',
					'Bridge username & Bridge password: Taken directly from the Mailbox configuration screen inside the Proton Mail Bridge app.'
				],
				playbooks: [
					{
						id: 'Proton',
						provider: 'Proton Mail Bridge (Local Desktop Setup)',
						steps: [
							'Download, install, and launch the official Proton Mail Bridge application on your computer (requires a paid Proton plan).',
							'Sign in to your Proton account inside Proton Mail Bridge and wait for initial synchronization to finish.',
							'Click on your account in the Bridge app and open Mailbox details / Configuration.',
							'Verify that the local IMAP port is 1143 and security mode is set to SSL/TLS or STARTTLS.',
							'Copy the Username and the Bridge Password displayed on that screen (never use your web Proton password).',
							'Click "Proton" in Quick fill, paste the Bridge username and Bridge password, and click Connect account.'
						]
					}
				]
			},
			localArchive: {
				title: 'Offline Archive (.mbox / .eml)',
				summary:
					'Import local email exports without connecting to an IMAP server or creating app passwords.',
				requirements: 'An exported .mbox mailbox archive or individual .eml message file on disk.',
				services: 'Google Takeout, Mozilla Thunderbird, Apple Mail exports, and standard RFC 5322 archives.',
				note: 'Full offline triage, search, and artifact extraction work locally; on-demand remote attachment fetches are unavailable.',
				fields: [
					'Archive label or email address: Identifies this archive account in your unified inbox (reusing an existing offline archive address appends messages).',
					'Archive format & file: Choose .mbox for multi-message exports or .eml for single messages, then select the file on disk.'
				],
				playbooks: [
					{
						id: 'Google Takeout',
						provider: 'Google Takeout & Gmail Export (.mbox)',
						steps: [
							'Open takeout.google.com, deselect all products except "Mail", and start an export.',
							'Download and extract the archive to locate the .mbox file (e.g. All mail Including Spam and Trash.mbox).',
							'Choose "Offline Archive", select the .mbox file, enter an archive email label, and click Import local archive.'
						]
					},
					{
						id: 'Thunderbird',
						provider: 'Mozilla Thunderbird & Local .eml Files',
						steps: [
							'Export a folder from Thunderbird as an .mbox file (or save individual messages as .eml files).',
							'Select the matching format (.mbox or .eml), choose the file, and run the streaming import.'
						]
					}
				]
			}
		},
		shell: {
			appTitle: 'triage-mail',
			commandPalette: 'Command Palette',
			searchPlaceholder: 'Search mail, senders, tags, codes…',
			globalAllMail: 'Global All Mail',
			inbox: 'Inbox',
			starred: 'Starred',
			snoozed: 'Snoozed',
			accounts: 'Accounts',
			smartViews: 'Smart Views',
			newSendersReview: 'New Senders Review',
			pending: 'Pending',
			declined: 'Declined',
			localCategories: 'Local Categories',
			savedViews: 'Saved Views',
			workspace: 'Workspace',
			navInbox: 'Unified Inbox',
			navDueDates: 'Due Dates',
			navAutomation: 'Automation',
			navOnboarding: 'Accounts & Vault',
			navSettings: 'Settings',
			loadedMessages: 'Loaded Messages',
			unreadCount: 'Unread',
			pendingBoletos: 'Pending Boletos',
			syncWorker: 'Sync Worker',
			individualMessages: 'Individual Messages',
			threadedConversations: 'Threaded Conversations',
			backToList: 'Back to list',
			allowRemoteImages: 'Allow remote images for this message',
			remoteImagesBlocked: 'Remote tracking pixels and external images are blocked by default.',
			remoteImagesAllowed: 'Remote images enabled for this message only.',
			summarizeLocalAi: 'Summarize with Local AI',
			attachmentsTitle: 'Attachments (On-Demand IMAP BODY.PEEK)',
			fetchAttachment: 'Fetch & Save',
			fetchingAttachment: 'Fetching…',
			snooze: 'Snooze',
			archive: 'Archive',
			trash: 'Trash',
			star: 'Star',
			unstar: 'Unstar',
			markRead: 'Mark Read',
			markUnread: 'Mark Unread',
			createRule: 'Create Rule',
			addTag: 'Add Local Tag',
			oneClickUnsubscribe: 'One-Click Unsubscribe (RFC 8058)',
			emptyScopeTitle: 'No messages match this scope',
			emptyScopeBody: 'Switch folders or clear your search filter to inspect other mail.',
			clearFilters: 'Clear filters',
			lexical: 'Lexical',
			meaningLocal: 'Meaning (local)',
			hybridLocal: 'Words + meaning (local)',
			onnxDisabledHint:
				'Local ONNX semantic search is disabled (experimental) — enable in Settings.',
			extractedArtifacts: 'Extracted Artifacts',
			copyCode: 'Copy Code',
			copyLine: 'Copy Digitable Line',
			copyPix: 'Copy Pix Payload',
			copyNfe: 'Copy NF-e Key',
			copyTracking: 'Copy Tracking',
			noMessageSelected:
				'Select a message to inspect its headers, extractions, and sandboxed body.',
			windowControls: 'Window controls',
			windowDrag: 'Drag window',
			windowMinimize: 'Minimize',
			windowMaximize: 'Maximize',
			windowRestore: 'Restore',
			windowClose: 'Close',
			windowMinimizedHint: 'Minimized to dock',
			windowRestoreAction: 'Restore window',
			windowClosedTitle: 'Workstation window closed',
			windowClosedBody:
				'Your local triage session is paused. Reopen the window to continue working.',
			windowReopen: 'Reopen triage-mail',
			openDrawer: 'Open navigation drawer',
			syncNow: 'Sync now',
			windowActionError: 'Window action unavailable.',
			blockRemoteImages: 'Block images again',
			downloaded: 'Downloaded',
			noSubject: '(No subject)',
			unknownSender: 'Unknown sender',
			loadingMessage: 'Loading message…',
			loadMessageError: 'Message could not be loaded. Please try again.',
			tryAgain: 'Try again',
			resetCategory: 'Reset to automatic',
			noDisplayableContent: 'This message has no displayable content.',
			attachmentUnavailableOffline:
				'Offline archive — attachment binary is not stored for on-demand fetch.',
			saveAttachmentAs: 'Save As…',
			savingAttachment: 'Saving…',
			savedAttachment: 'Saved',
			saveAttachmentError: 'Save failed. Please try again.',
			previewAttachment: 'Preview',
			previewingAttachment: 'Opening preview…',
			closePreview: 'Close preview',
			previewAttachmentUnsupported:
				'Preview is only supported for safe images, text, and PDFs up to 10 MiB.',
			previewAttachmentError: 'Preview failed. Save the file instead.',
			previewEphemeralNotice: 'Ephemeral in-memory preview — cleared immediately on close.'
		}
	},
	'pt-BR': {
		mailboxes: {
			mailboxes: 'Caixas de correio',
			navigation: 'Navegação de correio',
			allMail: 'Todos os emails',
			Inbox: 'Entrada',
			Starred: 'Com estrela',
			Snoozed: 'Adiados',
			spam: 'Spam',
			archive: 'Arquivo',
			trash: 'Lixeira',
			other: 'Outras pastas',
			loadingFolders: 'Carregando pastas…',
			catalogError: 'Pastas indisponíveis. Emails em cache continuam acessíveis.',
			notSynced: 'Pastas ainda não sincronizadas.',
			retry: 'Tentar novamente',
			unavailable: 'Esta pasta não está mais disponível. Escolha outra caixa.',
			noCached: 'Nenhuma mensagem em cache nesta visualização.',
			empty: 'Nenhuma mensagem encontrada',
			emptyHelp: 'Tente outra busca ou caixa de correio.',
			star: 'Marcar com estrela',
			unstar: 'Remover estrela',
			snooze: 'Adiar',
			unsnooze: 'Remover adiamento',
			snoozedUntil: 'Adiado até',
			snoozeHelp:
				'Escolha data e hora locais dentro do próximo ano. Salvo apenas neste dispositivo.',
			deadline: 'Lembrar em',
			saveSnooze: 'Salvar adiamento',
			invalidDeadline: 'Escolha uma data futura dentro de um ano.',
			actions: 'Ações da mensagem',
			close: 'Fechar',
			markRead: 'Marcar como lido',
			markUnread: 'Marcar como não lido',
			moveTrash: 'Mover para a lixeira',
			tag: 'Adicionar etiqueta local',
			unsubscribe: 'Enviar pedido de cancelamento da inscrição',
			searchScope: 'Buscar nesta caixa',
			settings: 'Configurações',
			dueDates: 'Contas a Vencer',
			noActions: 'Nenhuma ação encontrada.',
			actionSearch: 'Buscar uma ação da mensagem',
			saved: 'Salvo localmente. Alterações no servidor sincronizam em segundo plano.',
			localSaved: 'Salvo neste dispositivo.',
			saveError: 'Não foi possível salvar. Tente novamente.',
			loadError: 'Não foi possível carregar os emails. Tente novamente.',
			vaultError: 'Desbloqueie o cofre local para acessar os emails.',
			loading: 'Carregando mensagens…'
		},
		app: {
			title: 'Triage Mail',
			tagline: 'Armazenamento local. Credenciais privadas. Sua escolha de provedor.'
		},
		language: {
			en: 'EN',
			ptBR: 'PT',
			selectLabel: 'Selecionar idioma'
		},
		accountPhoto: {
			label: 'Foto da conta',
			choose: 'Escolher uma foto',
			replace: 'Substituir foto',
			remove: 'Remover foto',
			help: 'JPEG, PNG ou WebP local · até 2 MiB e 1024 × 1024 pixels.',
			pending: 'Atualizando foto…',
			loadError:
				'Não foi possível carregar a foto. Reabra as configurações de contas para tentar novamente.',
			saveError: 'Não foi possível salvar a foto. Escolha uma imagem válida ou tente novamente.',
			invalidFile: 'Escolha uma imagem não vazia de até 2 MiB.'
		},
		onboarding: {
			unlockTitle: 'Desbloqueie seu correio',
			accountsTitle: 'Suas contas',
			openingMail: 'Abrindo e-mails locais…',
			vaultLockedHelp:
				'Seu chaveiro do sistema não está disponível ou o cofre está bloqueado. Digite a frase secreta do cofre local. No primeiro uso, isso cria um cofre criptografado de fallback.',
			vaultPassphraseLabel: 'Frase secreta do cofre',
			unlocking: 'Desbloqueando…',
			unlockVault: 'Desbloquear cofre',
			retryKeyring: 'Tentar chaveiro do sistema novamente',
			tryAgain: 'Tentar abrir e-mails novamente',
			connectedAccounts: 'Contas conectadas',
			openInbox: 'Abrir caixa de entrada',
			connectFirstAccount: 'Conecte sua primeira conta para centralizar seus e-mails.',
			accountSaveError: 'Não foi possível salvar a conta',
			pageTitle: 'Desbloqueio do Cofre e Contas',
			vaultLockedBadge: 'Cofre Selado',
			vaultUnlockedBadge: 'Cofre Desbloqueado',
			recoveryModeBadge: 'Recuperação Disponível',
			demoPreviewBadge: 'Prévia Demo',
			accountsIndexed: 'contas indexadas',
			viewConnectedAccounts: 'Ver contas conectadas',
			addAccountAction: 'Adicionar conta',
			backToInbox: 'Voltar para Caixa',
			continueWithoutRestore: 'Continuar sem restaurar',
			restoreBackupAction: 'Restaurar de Backup Criptografado (.triagebak)',
			restoreBackupTitle: 'Restaurar Backup Criptografado',
			restoreBackupHelp:
				'Escolha um arquivo .triagebak criptografado e confirme sua frase secreta para restaurar os e-mails locais antes da inicialização.',
			restorePassphraseLabel: 'Frase secreta do backup (mín. 12 caracteres)',
			restoreConfirmLabel: 'Confirmar frase secreta do backup',
			restoreConsentLabel:
				'Entendo que isso restaura metadados criptografados e mantém automações desativadas até reativação.',
			restoreChooseFile: 'Escolher backup e restaurar',
			restorePassphraseMismatch:
				'A frase secreta e a confirmação devem coincidir e ter pelo menos 12 caracteres.',
			restoreSuccess: 'Backup criptografado restaurado. Carregando contas locais…',
			restoreError: 'Não foi possível restaurar o backup. Verifique o arquivo e a frase secreta.',
			credentialsZeroized: 'As credenciais são zeradas do DOM imediatamente após o envio.',
			manualNetworkOnly: 'Apenas ações de rede manuais',
			showPassphrase: 'Mostrar frase secreta',
			hidePassphrase: 'Ocultar frase secreta',
			foldersCount: 'Pastas',
			folderColumn: 'Pasta',
			roleColumn: 'Função',
			totalColumn: 'Total',
			unreadColumn: 'Não lidas',
			loadingFolders: 'Carregando catálogo de pastas…',
			noFolders: 'Nenhuma pasta sincronizada ainda.',
			foldersUnavailable: 'Catálogo de pastas indisponível.',
			noAccountsTitle: 'Nenhuma conta conectada ainda',
			noAccountsHelp:
				'Execute a descoberta ISPDB por domínio ou conecte uma caixa local Proton Bridge.',
			configureFirstAccount: 'Configurar Primeira Conta',
			vaultUnlockError: 'Não foi possível desbloquear o cofre. Confira sua frase secreta e tente novamente.',
			createVaultTitle: 'Criar frase secreta mestre local',
			createVaultBadge: 'Novo Cofre Local',
			vaultCreateHelp:
				'Defina uma frase secreta mestre local para criptografar seu cofre e banco de e-mails neste dispositivo.',
			vaultCreateKeyringUnavailableHelp:
				'Nenhum chaveiro do sistema (Secret Service) está disponível nesta sessão. Crie uma frase secreta mestre local para criptografar suas contas e banco de e-mails com Argon2id + AES-GCM. Guarde-a com segurança — ela não pode ser recuperada se perdida.',
			vaultConfirmPassphraseLabel: 'Confirmar frase secreta mestre',
			vaultPassphraseMinHint: 'Use pelo menos 8 caracteres.',
			vaultPassphraseMismatch:
				'A frase secreta e a confirmação devem coincidir e ter pelo menos 8 caracteres.',
			createVaultAction: 'Criar cofre e continuar',
			creatingVault: 'Criando cofre…',
			reauthorize: 'Reautorizar',
			reauthorizing: 'Reautorizando…',
			reauthorizeHelp:
				'Entre novamente no navegador para renovar os tokens OAuth sem apagar os e-mails indexados localmente.',
			reauthorizeClientOverrideHint:
				'Deixe ID do cliente e Chave secreta em branco para reutilizar as credenciais salvas no cofre.',
			reauthorizeSubmit: 'Iniciar login no navegador',
			reauthorizeCancel: 'Cancelar',
			reauthorizeSuccess:
				'Tokens OAuth renovados. A sincronização foi retomada sem alterar os e-mails locais.',
			reauthorizeError:
				'A reautorização não foi concluída. Confira a orientação abaixo e tente novamente.',
			accountDiagnosticBanner: 'Atenção necessária na conexão',
			planBadge: 'Plano / Selo',
			planAuto: 'Automático (cota IMAP)',
			planHidden: 'Oculto',
			planCustom: 'Personalizado',
			planCustomLabel: 'Rótulo personalizado (máx. 12 caracteres)',
			planApply: 'Aplicar',
			planError: 'Não foi possível salvar o selo. Tente novamente.',
			storageUsage: 'Armazenamento usado'
		},
		accountForm: {
			discoveryIntro:
				'Informe seu e-mail para encontrar configurações locais do provedor. Você também pode escolher um provedor ou configurar manualmente.',
			advancedSettings: 'Configurações avançadas de conexão',
			microsoftSetup: 'Configurar login Microsoft',
			microsoftSetupHelp:
				'Use o ID de cliente do seu próprio aplicativo desktop e autorize no navegador.',
			onlineLookup: 'Buscar configurações online',
			onlineLookupHelp:
				'Esta ação envia apenas o domínio do e-mail ao ISPDB do Mozilla Thunderbird e substitui as configurações de conexão. Não faz login nem testa servidores.',
			confirmProbe: 'Confirmar e testar',
			probeHelp:
				'Confirme o host abaixo para testar a conexão IMAP com TLS na porta 993. Nenhuma credencial é enviada.',
			discovering: 'Verificando configurações do provedor…',
			providerFound: 'Configurações sugeridas:',
			discoveryUnresolved:
				'Nenhuma configuração correspondente encontrada. Informe o host IMAP do seu provedor nas configurações avançadas.',
			discoveryInvalid: 'Informe um único endereço de e-mail válido.',
			discoveryUnavailable:
				'A busca de configurações está indisponível. Escolha um provedor ou configure manualmente.',
			probeSuccess: 'Conexão IMAP com TLS confirmada. O login ainda não foi testado.',
			probeFailed:
				'Não foi possível confirmar o IMAP com TLS. Confira o host e tente novamente ou configure manualmente.',
			gmailAppPasswordHelp:
				'Criar uma senha de aplicativo Google (requer verificação em duas etapas)',
			gmailRegularPasswordHint:
				'Isto não parece uma senha de aplicativo Google (16 letras). A senha normal do Gmail será rejeitada. Crie primeiro uma senha de aplicativo.',

			title: 'Conectar uma conta',
			subtitle: 'Use as configurações de conexão fornecidas pelo seu provedor de e-mail.',
			methodLabel: 'Método de autenticação',
			methodAppPassword: 'Senha de aplicativo',
			methodOAuthPublic: 'OAuth · PKCE Público',
			methodOAuthByo: 'OAuth · Suas credenciais de cliente',
			methodProton: 'Proton Mail Bridge',
			methodLocalArchive: 'Arquivo Offline',
			emailLabel: 'Endereço de e-mail ou usuário IMAP',
			emailLabelBridge: 'Nome de usuário no Bridge',
			hostLabel: 'Host IMAP',
			portLabel: 'Porta',
			oauthHelp:
				'Registre um cliente desktop com seu provedor usando redirecionamento de loopback em http://127.0.0.1/callback com porta dinâmica. A autorização abrirá no seu navegador.',
			authEndpointLabel: 'Endpoint de autorização (HTTPS)',
			tokenEndpointLabel: 'Endpoint de token (HTTPS)',
			scopesLabel: 'Escopos IMAP',
			scopesHelp:
				'Escopos separados por espaço. Inclua offline access se seu provedor oferecer suporte.',
			clientIdLabel: 'ID do cliente',
			clientSecretLabel: 'Chave secreta do cliente',
			bridgePasswordLabel: 'Senha do Bridge',
			appPasswordLabel: 'Senha de aplicativo',
			protonNotice:
				'Inicie o Proton Mail Bridge primeiro e selecione o modo SSL/TLS IMAP. Apenas endereços locais (loopback) são permitidos; TLS autoassinado local é aceito.',
			oauthVaultNotice:
				'Autorizar contata os endpoints do provedor configurado. Tokens e credenciais ficam salvos no cofre nativo.',
			connectVaultNotice:
				'Conectar salva as credenciais no cofre nativo e inicia a sincronização de entrada.',
			connecting: 'Conectando…',
			authorizeAndConnect: 'Autorizar e conectar',
			connectAccount: 'Conectar conta',
			testLogin: 'Testar login',
			testingLogin: 'Testando login…',
			testLoginSuccess:
				'Login verificado. As credenciais foram limpas — digite a senha novamente para conectar.',
			testLoginFailed:
				'O teste de login falhou. Verifique host, porta e credenciais.',
			diagnostics: {
				TlsHandshakeFailed: {
					title: 'Falha de certificado TLS ou porta',
					hint: 'Verifique o host e a porta IMAP (993 para IMAPS). Para Proton Mail Bridge, confirme que o Bridge está em execução em 127.0.0.1:1143 com modo SSL/TLS ativo.'
				},
				AuthenticationRejected: {
					title: 'E-mail ou senha rejeitados pelo provedor',
					hint: 'Confira seu e-mail e senha. Para Gmail, iCloud, Yahoo ou Zoho, ative a verificação em duas etapas e gere uma Senha de App dedicada de 16 caracteres em vez da senha normal.'
				},
				ImapDisabledHint: {
					title: 'Acesso IMAP desativado nesta conta',
					hint: 'Abra as configurações do seu webmail (ex.: Gmail Encaminhamento e POP/IMAP, Zoho Contas -> IMAP ou configurações UOL/BOL/Terra) e habilite o acesso IMAP antes de conectar.'
				},
				OAuthAccessDenied: {
					title: 'Provedor negou a autorização OAuth',
					hint: 'O acesso foi recusado ou bloqueado. Em projetos pessoais do Google Cloud em modo Teste, adicione seu e-mail em Tela de permissão OAuth -> Usuários de teste e aprove no navegador. Tokens do modo Teste expiram em 7 dias; publique a tela de permissão "Em produção" (gratuito para uso pessoal) para evitar reautorizações semanais.'
				},
				OAuthInvalidClient: {
					title: 'Configuração do cliente OAuth inválida',
					hint: 'Verifique seu ID do cliente, Chave secreta e endpoints. Tokens do modo Teste do Google também expiram em 7 dias; publique a tela de permissão "Em produção" para evitar isso. Crie um cliente do tipo "App para computador" (ou "Cliente público/nativo") e registre http://127.0.0.1/callback como URI de redirecionamento.'
				},
				OAuthCancelled: {
					title: 'Autorização cancelada',
					hint: 'O login no navegador foi cancelado antes de concluir. Inicie a autorização novamente quando estiver pronto.'
				},
				NetworkTimeout: {
					title: 'Tempo limite de conexão esgotado',
					hint: 'O servidor não respondeu a tempo. Verifique sua conexão com a internet, firewall/VPN e se o host e a porta IMAP estão acessíveis.'
				}
			},
			browserAuthPending:
				'Conclua a autorização no seu navegador. Esta tentativa expira em cinco minutos.',
			cancelAuth: 'Cancelar autorização',
			quickFillLabel: 'Preenchimento rápido:',
			importGoogleJson: 'Importar JSON do Google Cloud',
			importGoogleJsonHelp:
				'Carregue o arquivo client_secret_*.json (App para computador) direto no formulário.',
			importGoogleJsonSuccess:
				'Credenciais OAuth do Google Cloud carregadas. Informe seu e-mail e autorize.',
			importGoogleJsonWebError:
				'Este JSON é de aplicativo Web. Crie um ID do cliente OAuth do tipo "App para computador" no Google Cloud Console.',
			importGoogleJsonInvalidError:
				'Arquivo JSON OAuth inválido. Selecione o client_secret_*.json baixado em Credenciais no Google Cloud.',
			localArchiveNotice:
				'Importe mensagens de um arquivo .mbox local (Google Takeout, Thunderbird) ou .eml único totalmente offline. Não requer senha nem servidor IMAP.',
			localArchiveEmailLabel: 'Rótulo ou endereço de e-mail do arquivo',
			localArchiveFormatLabel: 'Formato do arquivo',
			localArchiveFormatMbox: '.mbox (caixa com várias mensagens)',
			localArchiveFormatEml: '.eml (mensagem RFC 5322 única)',
			localArchiveChooseFile: 'Escolher arquivo .mbox ou .eml',
			localArchiveNoFile: 'Nenhum arquivo selecionado',
			localArchiveSubmit: 'Importar arquivo local',
			localArchiveImporting: 'Importando arquivo…',
			localArchiveProgress: 'Mensagens importadas até agora:',
			localArchiveSuccess: 'Mensagens importadas para o arquivo local:',
			localArchiveNoticeFooter:
				'As mensagens importadas ficam indexadas localmente no banco SQLCipher. Novas importações no mesmo e-mail adicionam mensagens em sequência.',
			localArchiveConflictError:
				'Este endereço pertence a uma conta IMAP ativa ou recuperada. Escolha outro e-mail para arquivos locais.',
			localArchiveImportError:
				'Não foi possível importar o arquivo. Verifique o formato (.mbox ou .eml) e as permissões.',
			localArchiveBadge: 'Arquivo Offline (Somente Local)',
			allSignInMethods: 'Mais métodos de acesso',
			fewerSignInMethods: 'Menos métodos de acesso',
			recommendedOption: 'Recomendado',
			googleCloudJsonOption: 'JSON OAuth do Google Cloud',
			offlineImportOption: 'Importação Offline .mbox / .eml',
			customImapOption: 'IMAP Personalizado / Avançado',
			resolvedServerSummary: 'Configurações do servidor resolvidas',
			providerGuidanceGoogle:
				'Escolha Senha de Aplicativo (mais simples com 2FA), importe um JSON OAuth Desktop do Google Cloud ou importe um .mbox do Google Takeout offline.',
			providerGuidanceMicrosoft:
				'Use o login Microsoft (PKCE Público) com seu ID do cliente, uma Senha de Aplicativo ou importe um arquivo .mbox/.eml offline.',
			providerGuidanceAppPassword:
				'Entre com uma Senha de Aplicativo do provedor ou importe um arquivo .mbox/.eml offline.',
			providerGuidanceProton:
				'Conecte-se ao Proton Mail Bridge em execução local (127.0.0.1:1143) ou importe um arquivo .mbox/.eml offline.',
			providerGuidanceCustom:
				'Configure o host e a porta IMAP abaixo ou escolha uma importação .mbox/.eml offline.'
		},
		guide: {
			title: 'Como funcionam os métodos de login e passo a passo',
			toggleOpen: 'Ver guia explicativo',
			toggleClose: 'Ocultar guia',
			requirementsLabel: 'Requisitos:',
			servicesLabel: 'Serviços compatíveis:',
			fieldsTitle: 'O que preencher no formulário abaixo:',
			stepsTitle: 'Passo a passo: como configurar e obter as credenciais:',
			appPassword: {
				title: 'Senha de Aplicativo (App Password)',
				summary:
					'Gere uma senha dedicada de 16 caracteres no painel de segurança do seu e-mail. Não requer criação de projeto de desenvolvedor.',
				requirements:
					'Verificação em duas etapas (2FA) ativada na conta de e-mail e acesso IMAP habilitado nas configurações do webmail.',
				services:
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Zoho, UOL, BOL, Terra, Outlook.com e servidores IMAP próprios.',
				hostTip:
					'Hosts comuns: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, imap.zoho.com:993, imap.uol.com.br:993',
				fields: [
					'Endereço de e-mail: Seu e-mail completo (ex.: leitor@example.com).',
					'Host IMAP e Porta: Clique no botão do seu provedor em "Preenchimento rápido" (ou informe o host do servidor e a porta 993).',
					'Senha de aplicativo: A senha exclusiva de 16 caracteres gerada no painel de segurança do seu provedor (cole sem espaços).'
				],
				playbooks: [
					{
						id: 'Gmail',
						provider: 'Google / Gmail e Google Workspace',
						steps: [
							'Acesse a página de Segurança da Conta Google (myaccount.google.com/security) e ative a "Verificação em duas etapas" (2FA).',
							'Acesse "Senhas de app" (myaccount.google.com/apppasswords), digite o nome "Triage Mail" e clique em "Criar".',
							'Copie a senha de 16 letras exibida na tela (ignore os espaços).',
							'No Gmail pelo navegador (Configurações -> Ver todas as configurações -> Encaminhamento e POP/IMAP), confirme que o "Acesso IMAP" está ativado.',
							'Com "Gmail" selecionado no Preenchimento rápido, informe seu e-mail e cole a senha de 16 caracteres.'
						]
					},
					{
						id: 'iCloud',
						provider: 'Apple iCloud Mail',
						steps: [
							'Acesse appleid.apple.com (é obrigatório estar com a autenticação de dois fatores ativada).',
							'Vá em "Início de sessão e segurança" -> "Senhas específicas de apps" -> "Gerar uma senha de app".',
							'Informe o nome "Triage Mail" e copie a senha gerada no formato xxxx-xxxx-xxxx-xxxx.',
							'Clique em "iCloud" no Preenchimento rápido, digite seu e-mail iCloud e cole a senha gerada.'
						]
					},
					{
						id: 'Yahoo',
						provider: 'Yahoo Mail e AOL',
						steps: [
							'Acesse Informações da conta -> Segurança da conta -> Gerar e gerenciar senhas de app, crie uma para "Triage Mail" e copie os 16 caracteres.',
							'Clique em "Yahoo" no Preenchimento rápido, preencha seu e-mail e cole a Senha de aplicativo.'
						]
					},
					{
						id: 'Fastmail',
						provider: 'Fastmail',
						steps: [
							'Acesse Settings -> Privacy & Security -> Integrations -> New app password, marque acesso IMAP e copie a senha.',
							'Clique em "Fastmail" no Preenchimento rápido, preencha seu e-mail e cole a Senha de aplicativo.'
						]
					},
					{
						id: 'Zoho',
						provider: 'Zoho Mail',
						steps: [
							'Acesse accounts.zoho.com -> Segurança -> Senhas de aplicativo e gere uma senha dedicada para "Triage Mail" (requer 2FA).',
							'Em Zoho Mail Configurações -> Contas de e-mail -> IMAP, confirme que o acesso IMAP está ativado.',
							'Clique em "Zoho" no Preenchimento rápido, digite seu e-mail e cole a Senha de aplicativo gerada.'
						]
					},
					{
						id: 'UOL',
						provider: 'UOL Mail',
						steps: [
							'Acesse Segurança / Minha Conta do UOL, habilite o acesso IMAP no webmail e gere uma Senha de app caso a verificação em duas etapas esteja ativa.',
							'Clique em "UOL" no Preenchimento rápido (imap.uol.com.br:993), informe seu e-mail @uol.com.br e cole sua senha.'
						]
					},
					{
						id: 'BOL',
						provider: 'BOL Mail',
						steps: [
							'Confirme que o acesso IMAP está ativado nas configurações do webmail BOL (e gere uma Senha de app se o 2FA estiver ativo).',
							'Clique em "BOL" no Preenchimento rápido (imap.bol.com.br:993), informe seu e-mail @bol.com.br e cole sua senha.'
						]
					},
					{
						id: 'Terra',
						provider: 'Terra Mail',
						steps: [
							'Verifique se o acesso IMAP está habilitado no Terra Mail e gere uma Senha de app caso a verificação em duas etapas esteja ativa.',
							'Clique em "Terra" no Preenchimento rápido (imap.terra.com.br:993), informe seu e-mail @terra.com.br e cole sua senha.'
						]
					}
				]
			},
			oauthPublic: {
				title: 'OAuth · PKCE Público',
				summary:
					'Login via navegador na página oficial do provedor com retorno local seguro (RFC 7636). Não precisa de chave secreta.',
				requirements:
					'Provedor deve aceitar clientes desktop com fluxo PKCE e redirecionamento para http://127.0.0.1/callback.',
				services: 'Microsoft 365, Outlook.com, Hotmail e provedores corporativos OIDC compatíveis.',
				redirectTip:
					'O Google bloqueia PKCE público puro sem certificação CASA; para Gmail utilize Senha de Aplicativo ou BYO Credentials.',
				fields: [
					'Preenchimento rápido (Outlook): Preenche automaticamente Host (outlook.office365.com), Porta (993), Endpoint de autorização, Endpoint de token e Escopos IMAP.',
					'ID do cliente (Client ID): O identificador público (UUID) do aplicativo registrado no portal Microsoft Entra / Azure.'
				],
				playbooks: [
					{
						id: 'Outlook',
						provider: 'Microsoft (Outlook.com / Hotmail / Microsoft 365)',
						steps: [
							'Acesse o portal Microsoft Entra / Azure (portal.azure.com -> Registros de aplicativo / App registrations) e clique em "Novo registro".',
							'Dê o nome "Triage Mail" e, em Tipos de conta com suporte, escolha "Contas em qualquer diretório organizacional e contas pessoais da Microsoft".',
							'Em URI de redirecionamento (Redirect URI), selecione a plataforma "Cliente público/nativo (móvel e área de trabalho)" e digite http://127.0.0.1/callback.',
							'Em Permissões de API -> Adicionar uma permissão -> APIs da Microsoft, adicione as permissões delegadas IMAP.AccessAsUser.All e offline_access.',
							'Na página "Visão geral" (Overview) do aplicativo criado, copie o "ID do aplicativo (cliente)" (Application / client ID).',
							'Clique em "Outlook" no Preenchimento rápido, informe seu e-mail, cole o ID do cliente e clique em "Autorizar e conectar".'
						]
					}
				]
			},
			oauthByo: {
				title: 'OAuth · Suas credenciais (BYO)',
				summary:
					'Insira seu próprio client_id e client_secret gerados no console do provedor para renovação contínua de tokens.',
				requirements:
					'Criar um projeto e credenciais do tipo "App para computador" no Google Cloud Console ou provedor corporativo.',
				services:
					'Google Cloud Console (Gmail / Google Workspace) e SSO corporativo (Keycloak, Okta).',
				gcpTip: 'Escopo necessário: https://mail.google.com/ com redirecionamento loopback. Tokens de atualização do modo Teste do Google expiram em 7 dias: publique a tela de permissão "Em produção" (gratuito para uso pessoal).',
				fields: [
					'Preenchimento rápido (Gmail OAuth): Preenche automaticamente Host (imap.gmail.com), Porta (993), Endpoints OAuth do Google e o escopo https://mail.google.com/.',
					'ID do cliente e Chave secreta: As credenciais OAuth de aplicativo Desktop geradas no seu projeto do Google Cloud Console.'
				],
				playbooks: [
					{
						id: 'Gmail OAuth',
						provider: 'Google Cloud Console (Gmail / Google Workspace)',
						steps: [
							'Acesse console.cloud.google.com, crie um novo projeto e, em "APIs e Serviços -> Biblioteca", pesquise por "Gmail API" e clique em "Ativar".',
							'Em "APIs e Serviços -> Tela de permissão OAuth", configure como Externo (ou Interno no Workspace), adicione o escopo https://mail.google.com/ e adicione seu próprio e-mail em "Usuários de teste" (Test users).',
							'Vá em "Credenciais -> Criar credenciais -> ID do cliente OAuth" e selecione o tipo de aplicativo "App para computador" (Desktop app).',
							'Baixe o arquivo client_secret_*.json (ou copie o "ID do cliente" e a "Chave secreta do cliente").',
							'Clique em "Gmail OAuth" (ou "Google Workspace") no Preenchimento rápido, use "Importar JSON do Google Cloud" para preencher automaticamente (ou cole manualmente), digite seu e-mail e clique em "Autorizar e conectar".',
							'Na janela do navegador, faça login e aprove o acesso (se o Google exibir o aviso de app não verificado para seu projeto pessoal, clique em Avançado -> Acessar).'
						]
					}
				]
			},
			proton: {
				title: 'Proton Mail Bridge',
				summary:
					'Conecta ao aplicativo oficial Proton Mail Bridge rodando localmente (127.0.0.1:1143) para descriptografar e ler mensagens.',
				requirements:
					'Plano pago do Proton Mail e aplicativo Proton Mail Bridge instalado e aberto em segundo plano.',
				services: 'Exclusivo para contas Proton Mail.',
				warning:
					'ATENÇÃO: Utilize o nome de usuário e a senha gerados dentro do aplicativo Bridge, e NUNCA a senha principal da sua conta Proton.',
				fields: [
					'Host e Porta: Fixados em 127.0.0.1 e 1143 (preenchidos automaticamente ao selecionar Proton).',
					'Nome de usuário no Bridge e Senha do Bridge: Credenciais locais copiadas diretamente da janela de configuração do Proton Mail Bridge.'
				],
				playbooks: [
					{
						id: 'Proton',
						provider: 'Proton Mail Bridge (Configuração Local no Computador)',
						steps: [
							'Baixe, instale e abra o aplicativo oficial Proton Mail Bridge no seu computador (requer assinatura Proton paga).',
							'Faça login na sua conta Proton dentro do Bridge e aguarde a sincronização inicial concluir.',
							'Clique sobre sua conta conectada no Bridge e abra "Detalhes da caixa de correio" (Mailbox details / Configuration).',
							'Confirme que a porta IMAP local é 1143 e que o modo de segurança está definido como SSL/TLS ou STARTTLS.',
							'Copie o "Username" e o "Password" mostrados nessa tela do Bridge (nunca utilize a senha da sua conta Proton da web).',
							'Clique em "Proton" no Preenchimento rápido, cole o usuário e a senha do Bridge e clique em "Conectar conta".'
						]
					}
				]
			},
			localArchive: {
				title: 'Arquivo Offline (.mbox / .eml)',
				summary:
					'Importe exportações locais de e-mail sem conectar a um servidor IMAP ou criar senhas de app.',
				requirements: 'Um arquivo de caixa .mbox exportado ou um arquivo de mensagem .eml no disco.',
				services: 'Google Takeout, Mozilla Thunderbird, Apple Mail e arquivos padrão RFC 5322.',
				note: 'Triagem offline, busca e extração de artefatos funcionam localmente; busca sob demanda de anexos remotos fica indisponível.',
				fields: [
					'Rótulo ou e-mail do arquivo: Identifica este arquivo na sua caixa unificada (reutilizar o mesmo e-mail de arquivo offline anexa mensagens em sequência).',
					'Formato e arquivo: Escolha .mbox para caixas exportadas ou .eml para mensagens únicas e selecione o arquivo no disco.'
				],
				playbooks: [
					{
						id: 'Google Takeout',
						provider: 'Google Takeout e Exportação do Gmail (.mbox)',
						steps: [
							'Acesse takeout.google.com, desmarque todos os produtos exceto "E-mail" e inicie a exportação.',
							'Baixe e extraia o pacote para localizar o arquivo .mbox (ex.: Todos os e-mails.mbox).',
							'Escolha "Arquivo Offline", selecione o arquivo .mbox, informe um rótulo/e-mail e clique em Importar arquivo local.'
						]
					},
					{
						id: 'Thunderbird',
						provider: 'Mozilla Thunderbird e Arquivos .eml Locais',
						steps: [
							'Exporte uma pasta do Thunderbird em formato .mbox (ou salve mensagens individuais em .eml).',
							'Selecione o formato correspondente (.mbox ou .eml), escolha o arquivo e execute a importação em fluxo.'
						]
					}
				]
			}
		},
		shell: {
			appTitle: 'triage-mail',
			commandPalette: 'Paleta de Comandos',
			searchPlaceholder: 'Buscar e-mails, remetentes, tags, códigos…',
			globalAllMail: 'Todos os E-mails',
			inbox: 'Entrada',
			starred: 'Com Estrela',
			snoozed: 'Adiados',
			accounts: 'Contas',
			smartViews: 'Visões Inteligentes',
			newSendersReview: 'Novos Remetentes',
			pending: 'Pendentes',
			declined: 'Recusados',
			localCategories: 'Categorias Locais',
			savedViews: 'Visões Salvas',
			workspace: 'Estação',
			navInbox: 'Caixa Unificada',
			navDueDates: 'Vencimentos',
			navAutomation: 'Automação',
			navOnboarding: 'Contas e Cofre',
			navSettings: 'Configurações',
			loadedMessages: 'Carregadas',
			unreadCount: 'Não lidas',
			pendingBoletos: 'Boletos Pendentes',
			syncWorker: 'Sincronização',
			individualMessages: 'Mensagens Individuais',
			threadedConversations: 'Conversas Agrupadas',
			backToList: 'Voltar para lista',
			allowRemoteImages: 'Permitir imagens remotas nesta mensagem',
			remoteImagesBlocked: 'Pixels de rastreamento e imagens externas são bloqueados por padrão.',
			remoteImagesAllowed: 'Imagens remotas liberadas apenas para esta mensagem.',
			summarizeLocalAi: 'Resumir com IA Local',
			attachmentsTitle: 'Anexos (IMAP BODY.PEEK sob demanda)',
			fetchAttachment: 'Baixar sob demanda',
			fetchingAttachment: 'Buscando…',
			snooze: 'Adiar',
			archive: 'Arquivar',
			trash: 'Lixeira',
			star: 'Favoritar',
			unstar: 'Remover estrela',
			markRead: 'Marcar como lida',
			markUnread: 'Marcar como não lida',
			createRule: 'Criar Regra',
			addTag: 'Adicionar Tag Local',
			oneClickUnsubscribe: 'Cancelar Inscrição (RFC 8058)',
			emptyScopeTitle: 'Nenhuma mensagem neste filtro',
			emptyScopeBody: 'Altere a pasta ou limpe a busca para visualizar outras mensagens.',
			clearFilters: 'Limpar filtros',
			lexical: 'Lexical',
			meaningLocal: 'Semântica (local)',
			hybridLocal: 'Palavras + semântica (local)',
			onnxDisabledHint:
				'Busca semântica ONNX local desativada (experimental) — ative em Configurações.',
			extractedArtifacts: 'Artefatos Extraídos',
			copyCode: 'Copiar Código',
			copyLine: 'Copiar Linha Digitável',
			copyPix: 'Copiar Copia e Cola Pix',
			copyNfe: 'Copiar Chave NF-e',
			copyTracking: 'Copiar Rastreio',
			noMessageSelected:
				'Selecione uma mensagem para inspecionar cabeçalhos, extrações e corpo isolado.',
			windowControls: 'Controles da janela',
			windowDrag: 'Arrastar janela',
			windowMinimize: 'Minimizar',
			windowMaximize: 'Maximizar',
			windowRestore: 'Restaurar',
			windowClose: 'Fechar',
			windowMinimizedHint: 'Minimizado na barra',
			windowRestoreAction: 'Restaurar janela',
			windowClosedTitle: 'Janela da estação fechada',
			windowClosedBody: 'Sua sessão local de triagem foi pausada. Reabra a janela para continuar.',
			windowReopen: 'Reabrir triage-mail',
			openDrawer: 'Abrir menu de navegação',
			syncNow: 'Sincronizar agora',
			windowActionError: 'Ação da janela indisponível.',
			blockRemoteImages: 'Bloquear imagens novamente',
			downloaded: 'Baixado',
			noSubject: '(Sem assunto)',
			unknownSender: 'Remetente desconhecido',
			loadingMessage: 'Carregando mensagem…',
			loadMessageError: 'Não foi possível carregar a mensagem. Tente novamente.',
			tryAgain: 'Tentar novamente',
			resetCategory: 'Restaurar automático',
			noDisplayableContent: 'Esta mensagem não possui conteúdo exibível.',
			attachmentUnavailableOffline:
				'Arquivo offline — o binário do anexo não está disponível para busca sob demanda.',
			saveAttachmentAs: 'Salvar como…',
			savingAttachment: 'Salvando…',
			savedAttachment: 'Salvo',
			saveAttachmentError: 'Falha ao salvar. Tente novamente.',
			previewAttachment: 'Visualizar',
			previewingAttachment: 'Abrindo visualização…',
			closePreview: 'Fechar visualização',
			previewAttachmentUnsupported:
				'A visualização é suportada apenas para imagens seguras, texto e PDFs de até 10 MiB.',
			previewAttachmentError: 'Falha ao visualizar. Salve o arquivo em vez disso.',
			previewEphemeralNotice:
				'Visualização efêmera em memória — apagada imediatamente ao fechar.'
		}
	}
};
