import type { SupportedLocale, Translations } from './types';

export const translations: Record<SupportedLocale, Translations> = {
	en: {
		app: {
			title: 'Triage Mail',
			tagline: 'Local storage. Private credentials. Your choice of provider.'
		},
		language: {
			en: 'EN',
			ptBR: 'PT',
			selectLabel: 'Select language'
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
			accountSaveError: 'Account could not be saved'
		},
		accountForm: {
			title: 'Connect an account',
			subtitle: 'Use the connection settings supplied by your mail provider.',
			methodLabel: 'Sign-in method',
			methodAppPassword: 'App password',
			methodOAuthPublic: 'OAuth · Public PKCE',
			methodOAuthByo: 'OAuth · Your client credentials',
			methodProton: 'Proton Mail Bridge',
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
			browserAuthPending:
				'Finish authorization in your browser. This attempt expires in five minutes.',
			cancelAuth: 'Cancel authorization',
			quickFillLabel: 'Quick fill:'
		},
		guide: {
			title: 'How sign-in methods work & requirements',
			toggleOpen: 'Show guide',
			toggleClose: 'Hide guide',
			requirementsLabel: 'Requirements:',
			servicesLabel: 'Supported services:',
			appPassword: {
				title: 'App Password',
				summary:
					'Generate a 16-character dedicated password in your email security panel. Does not require developer setups.',
				requirements:
					'Two-Factor Authentication (2FA) must be enabled on your account. IMAP must be turned on in webmail settings.',
				services:
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Outlook.com, and custom IMAP servers.',
				hostTip:
					'Popular hosts: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, outlook.office365.com:993'
			},
			oauthPublic: {
				title: 'OAuth · Public PKCE',
				summary:
					'Browser login via official provider page with local loopback callback (RFC 7636). Does not need a client secret.',
				requirements:
					'Provider must support public desktop PKCE clients and http://127.0.0.1/callback redirects.',
				services:
					'Microsoft 365, Outlook.com, Hotmail, and RFC 7636 compliant identity providers.',
				redirectTip:
					'Google blocks pure public PKCE for desktop IMAP without CASA certification; for Gmail use App Password or BYO Credentials.'
			},
			oauthByo: {
				title: 'OAuth · BYO Credentials',
				summary:
					'Bring your own developer client_id and client_secret for continuous OAuth token refresh.',
				requirements:
					'Create a Desktop Application in Google Cloud Console or your enterprise identity provider.',
				services:
					'Google Cloud Console (Gmail / Google Workspace) and private enterprise SSO (Keycloak, Okta).',
				gcpTip: 'Scope required: https://mail.google.com/ with loopback redirect.'
			},
			proton: {
				title: 'Proton Mail Bridge',
				summary:
					'Connects to the official Proton Mail Bridge app running locally on your computer (127.0.0.1:1143) to read zero-access encrypted email.',
				requirements:
					'Active paid Proton subscription and Proton Mail Bridge app installed and running in the background.',
				services: 'Proton Mail exclusively.',
				warning:
					'IMPORTANT: Use the username and password displayed inside the Proton Mail Bridge app, NOT your master Proton account password.'
			}
		}
	},
	'pt-BR': {
		app: {
			title: 'Triage Mail',
			tagline: 'Armazenamento local. Credenciais privadas. Sua escolha de provedor.'
		},
		language: {
			en: 'EN',
			ptBR: 'PT',
			selectLabel: 'Selecionar idioma'
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
			accountSaveError: 'Não foi possível salvar a conta'
		},
		accountForm: {
			title: 'Conectar uma conta',
			subtitle: 'Use as configurações de conexão fornecidas pelo seu provedor de e-mail.',
			methodLabel: 'Método de autenticação',
			methodAppPassword: 'Senha de aplicativo',
			methodOAuthPublic: 'OAuth · PKCE Público',
			methodOAuthByo: 'OAuth · Suas credenciais de cliente',
			methodProton: 'Proton Mail Bridge',
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
			browserAuthPending:
				'Conclua a autorização no seu navegador. Esta tentativa expira em cinco minutos.',
			cancelAuth: 'Cancelar autorização',
			quickFillLabel: 'Preenchimento rápido:'
		},
		guide: {
			title: 'Como funcionam os métodos de login e requisitos',
			toggleOpen: 'Ver guia explicativo',
			toggleClose: 'Ocultar guia',
			requirementsLabel: 'Requisitos:',
			servicesLabel: 'Serviços compatíveis:',
			appPassword: {
				title: 'Senha de Aplicativo (App Password)',
				summary:
					'Gere uma senha dedicada de 16 caracteres no painel de segurança do seu e-mail. Não requer criação de projeto de desenvolvedor.',
				requirements:
					'Verificação em duas etapas (2FA) ativada na conta de e-mail e acesso IMAP habilitado nas configurações do webmail.',
				services:
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Outlook.com e servidores IMAP próprios.',
				hostTip:
					'Hosts comuns: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, outlook.office365.com:993'
			},
			oauthPublic: {
				title: 'OAuth · PKCE Público',
				summary:
					'Login via navegador na página oficial do provedor com retorno local seguro (RFC 7636). Não precisa de chave secreta.',
				requirements:
					'Provedor deve aceitar clientes desktop com fluxo PKCE e redirecionamento para http://127.0.0.1/callback.',
				services:
					'Microsoft 365, Outlook.com, Hotmail e provedores corporativos OIDC compatíveis.',
				redirectTip:
					'O Google bloqueia PKCE público puro sem certificação CASA; para Gmail utilize Senha de Aplicativo ou BYO Credentials.'
			},
			oauthByo: {
				title: 'OAuth · Suas credenciais (BYO)',
				summary:
					'Insira seu próprio client_id e client_secret gerados no console do provedor para renovação contínua de tokens.',
				requirements:
					'Criar um projeto e credenciais do tipo "App para computador" no Google Cloud Console ou provedor corporativo.',
				services:
					'Google Cloud Console (Gmail / Google Workspace) e SSO corporativo (Keycloak, Okta).',
				gcpTip: 'Escopo necessário: https://mail.google.com/ com redirecionamento loopback.'
			},
			proton: {
				title: 'Proton Mail Bridge',
				summary:
					'Conecta ao aplicativo oficial Proton Mail Bridge rodando localmente (127.0.0.1:1143) para descriptografar e ler mensagens.',
				requirements:
					'Plano pago do Proton Mail e aplicativo Proton Mail Bridge instalado e aberto em segundo plano.',
				services: 'Exclusivo para contas Proton Mail.',
				warning:
					'ATENÇÃO: Utilize o nome de usuário e a senha gerados dentro do aplicativo Bridge, e NUNCA a senha principal da sua conta Proton.'
			}
		}
	}
};
