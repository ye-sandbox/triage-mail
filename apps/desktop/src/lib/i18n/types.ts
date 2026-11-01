export type SupportedLocale = 'en' | 'pt-BR';

export interface GuidePlaybook {
	id?: string;
	provider: string;
	steps: string[];
}

export interface Translations {
	app: {
		title: string;
		tagline: string;
	};
	language: {
		en: string;
		ptBR: string;
		selectLabel: string;
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
