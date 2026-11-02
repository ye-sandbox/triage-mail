export type SupportedLocale = 'en' | 'pt-BR';

export interface GuidePlaybook {
	id?: string;
	provider: string;
	steps: string[];
}

export interface Translations {
	mailboxes: {
		mailboxes: string;
		navigation: string;
		allMail: string;
		Inbox: string;
		Starred: string;
		Snoozed: string;
		spam: string;
		archive: string;
		trash: string;
		other: string;
		loadingFolders: string;
		catalogError: string;
		notSynced: string;
		retry: string;
		unavailable: string;
		noCached: string;
		empty: string;
		emptyHelp: string;
		star: string;
		unstar: string;
		snooze: string;
		unsnooze: string;
		snoozedUntil: string;
		snoozeHelp: string;
		deadline: string;
		saveSnooze: string;
		invalidDeadline: string;
		actions: string;
		close: string;
		markRead: string;
		markUnread: string;
		moveTrash: string;
		tag: string;
		unsubscribe: string;
		searchScope: string;
		settings: string;
		noActions: string;
		actionSearch: string;
		saved: string;
		localSaved: string;
		saveError: string;
		loadError: string;
		vaultError: string;
		loading: string;
	};
	app: {
		title: string;
		tagline: string;
	};
	language: {
		en: string;
		ptBR: string;
		selectLabel: string;
	};
	accountPhoto: {
		label: string;
		choose: string;
		replace: string;
		remove: string;
		help: string;
		pending: string;
		loadError: string;
		saveError: string;
		invalidFile: string;
	};
	onboarding: {
		unlockTitle: string;
		accountsTitle: string;
		openingMail: string;
		vaultLockedHelp: string;
		vaultPassphraseLabel: string;
		unlocking: string;
		unlockVault: string;
		retryKeyring: string;
		tryAgain: string;
		connectedAccounts: string;
		openInbox: string;
		connectFirstAccount: string;
		accountSaveError: string;
	};
	accountForm: {
		discoveryIntro: string;
		advancedSettings: string;
		microsoftSetup: string;
		microsoftSetupHelp: string;
		onlineLookup: string;
		onlineLookupHelp: string;
		confirmProbe: string;
		probeHelp: string;
		discovering: string;
		providerFound: string;
		discoveryUnresolved: string;
		discoveryInvalid: string;
		discoveryUnavailable: string;
		probeSuccess: string;
		probeFailed: string;
		gmailAppPasswordHelp: string;

		title: string;
		subtitle: string;
		methodLabel: string;
		methodAppPassword: string;
		methodOAuthPublic: string;
		methodOAuthByo: string;
		methodProton: string;
		emailLabel: string;
		emailLabelBridge: string;
		hostLabel: string;
		portLabel: string;
		oauthHelp: string;
		authEndpointLabel: string;
		tokenEndpointLabel: string;
		scopesLabel: string;
		scopesHelp: string;
		clientIdLabel: string;
		clientSecretLabel: string;
		bridgePasswordLabel: string;
		appPasswordLabel: string;
		protonNotice: string;
		oauthVaultNotice: string;
		connectVaultNotice: string;
		connecting: string;
		authorizeAndConnect: string;
		connectAccount: string;
		browserAuthPending: string;
		cancelAuth: string;
		quickFillLabel: string;
	};
	guide: {
		title: string;
		toggleOpen: string;
		toggleClose: string;
		requirementsLabel: string;
		servicesLabel: string;
		fieldsTitle: string;
		stepsTitle: string;
		appPassword: {
			title: string;
			summary: string;
			requirements: string;
			services: string;
			hostTip: string;
			fields: string[];
			playbooks: GuidePlaybook[];
		};
		oauthPublic: {
			title: string;
			summary: string;
			requirements: string;
			services: string;
			redirectTip: string;
			fields: string[];
			playbooks: GuidePlaybook[];
		};
		oauthByo: {
			title: string;
			summary: string;
			requirements: string;
			services: string;
			gcpTip: string;
			fields: string[];
			playbooks: GuidePlaybook[];
		};
		proton: {
			title: string;
			summary: string;
			requirements: string;
			services: string;
			warning: string;
			fields: string[];
			playbooks: GuidePlaybook[];
		};
	};
	settingsModal: {
		title: string;
		close: string;
		navAria: string;
		tabs: {
			rules: string;
			general: string;
			accounts: string;
			search: string;
			ai: string;
			privacy: string;
		};
		unsavedChanges: string;
		stay: string;
		discard: string;
	};
}
