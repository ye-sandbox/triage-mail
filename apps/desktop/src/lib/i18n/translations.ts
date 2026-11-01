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
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Outlook.com, and custom IMAP servers.',
				hostTip:
					'Popular hosts: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, outlook.office365.com:993',
				fields: [
					'Email address: Your full email address (e.g. reader@example.com).',
					'IMAP host & Port: Click your provider in Quick fill below (or enter your server host and port 993).',
					'App password: The 16-character password generated in your provider security settings (paste without spaces).'
				],
				playbooks: [
					{
						provider: 'Google / Gmail & Google Workspace',
						steps: [
							'Open your Google Account security page (myaccount.google.com/security) and enable 2-Step Verification (2FA).',
							'Go to App passwords (myaccount.google.com/apppasswords), enter the name "Triage Mail", and click Create.',
							'Copy the 16-letter password shown on screen (ignore spaces).',
							'In Gmail on the web (Settings -> See all settings -> Forwarding and POP/IMAP), verify that IMAP access is Enabled.',
							'Click "Gmail" in Quick fill below, enter your email, and paste the 16-character App Password.'
						]
					},
					{
						provider: 'Apple iCloud Mail',
						steps: [
							'Sign in to appleid.apple.com (Two-Factor Authentication is required).',
							'Open Sign-In and Security -> App-Specific Passwords -> Generate an app-specific password.',
							'Name it "Triage Mail" and copy the generated xxxx-xxxx-xxxx-xxxx password.',
							'Click "iCloud" in Quick fill below, enter your iCloud email address, and paste the generated password.'
						]
					},
					{
						provider: 'Yahoo Mail, AOL & Fastmail',
						steps: [
							'Yahoo / AOL: Go to Account Info -> Account Security -> Generate and manage app passwords, create one for "Triage Mail", and copy it.',
							'Fastmail: Go to Settings -> Privacy & Security -> Integrations -> New app password, select IMAP access, and copy the password.',
							'Click "Yahoo" or "Fastmail" in Quick fill below, enter your email, and paste the App Password.'
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
				services:
					'Microsoft 365, Outlook.com, Hotmail, and RFC 7636 compliant identity providers.',
				redirectTip:
					'Google blocks pure public PKCE for desktop IMAP without CASA certification; for Gmail use App Password or BYO Credentials.',
				fields: [
					'Quick fill (Outlook): Automatically populates Host (outlook.office365.com), Port (993), Authorization endpoint, Token endpoint, and IMAP scopes.',
					'Client ID: The Application (client) UUID generated in your Microsoft Entra / Azure app registration.'
				],
				playbooks: [
					{
						provider: 'Microsoft (Outlook.com / Hotmail / Microsoft 365)',
						steps: [
							'Open Microsoft Entra / Azure Portal (portal.azure.com -> App registrations) and click New registration.',
							'Enter a name ("Triage Mail") and select "Accounts in any organizational directory and personal Microsoft accounts".',
							'Under Redirect URI, choose platform "Public client/native (mobile & desktop)" and enter http://127.0.0.1/callback.',
							'In API permissions -> Add a permission -> Microsoft APIs, add delegated permissions IMAP.AccessAsUser.All and offline_access.',
							'On the Overview page of your registered app, copy the Application (client) ID.',
							'Click "Outlook" in Quick fill below, type your email, paste the Client ID, and click Authorize and connect.'
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
				gcpTip: 'Scope required: https://mail.google.com/ with loopback redirect.',
				fields: [
					'Quick fill (Gmail OAuth): Auto-fills Host (imap.gmail.com), Port (993), Google OAuth endpoints, and the https://mail.google.com/ scope.',
					'Client ID & Client secret: The Desktop OAuth client credentials created in your Google Cloud Console project.'
				],
				playbooks: [
					{
						provider: 'Google Cloud Console (Gmail / Google Workspace)',
						steps: [
							'Open console.cloud.google.com, create a project, and enable the "Gmail API" under APIs & Services -> Library.',
							'Go to APIs & Services -> OAuth consent screen, choose External (or Internal for Workspace), add scope https://mail.google.com/, and add your own email under Test users.',
							'Go to Credentials -> Create Credentials -> OAuth client ID and select Application type "Desktop app".',
							'Copy both the generated Client ID and Client secret.',
							'Click "Gmail OAuth" in Quick fill below, enter your email, paste your Client ID and Client secret, and click Authorize and connect.',
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
						provider: 'Proton Mail Bridge (Local Desktop Setup)',
						steps: [
							'Download, install, and launch the official Proton Mail Bridge application on your computer (requires a paid Proton plan).',
							'Sign in to your Proton account inside Proton Mail Bridge and wait for initial synchronization to finish.',
							'Click on your account in the Bridge app and open Mailbox details / Configuration.',
							'Verify that the local IMAP port is 1143 and security mode is set to SSL/TLS or STARTTLS.',
							'Copy the Username and the Bridge Password displayed on that screen (never use your web Proton password).',
							'Click "Proton" in Quick fill below, paste the Bridge username and Bridge password, and click Connect account.'
						]
					}
				]
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
					'Gmail, Google Workspace, iCloud, Yahoo, AOL, Fastmail, Outlook.com e servidores IMAP próprios.',
				hostTip:
					'Hosts comuns: imap.gmail.com:993, imap.mail.me.com:993, imap.mail.yahoo.com:993, outlook.office365.com:993',
				fields: [
					'Endereço de e-mail: Seu e-mail completo (ex.: leitor@example.com).',
					'Host IMAP e Porta: Clique no botão do seu provedor em "Preenchimento rápido" abaixo (ou informe o host do servidor e a porta 993).',
					'Senha de aplicativo: A senha exclusiva de 16 caracteres gerada no painel de segurança do seu provedor (cole sem espaços).'
				],
				playbooks: [
					{
						provider: 'Google / Gmail e Google Workspace',
						steps: [
							'Acesse a página de Segurança da Conta Google (myaccount.google.com/security) e ative a "Verificação em duas etapas" (2FA).',
							'Acesse "Senhas de app" (myaccount.google.com/apppasswords), digite o nome "Triage Mail" e clique em "Criar".',
							'Copie a senha de 16 letras exibida na tela (ignore os espaços).',
							'No Gmail pelo navegador (Configurações -> Ver todas as configurações -> Encaminhamento e POP/IMAP), confirme que o "Acesso IMAP" está ativado.',
							'Clique em "Gmail" no Preenchimento rápido abaixo, informe seu e-mail e cole a senha de 16 caracteres.'
						]
					},
					{
						provider: 'Apple iCloud Mail',
						steps: [
							'Acesse appleid.apple.com (é obrigatório estar com a autenticação de dois fatores ativada).',
							'Vá em "Início de sessão e segurança" -> "Senhas específicas de apps" -> "Gerar uma senha de app".',
							'Informe o nome "Triage Mail" e copie a senha gerada no formato xxxx-xxxx-xxxx-xxxx.',
							'Clique em "iCloud" no Preenchimento rápido abaixo, digite seu e-mail iCloud e cole a senha gerada.'
						]
					},
					{
						provider: 'Yahoo Mail, AOL e Fastmail',
						steps: [
							'Yahoo / AOL: Acesse Informações da conta -> Segurança da conta -> Gerar e gerenciar senhas de app, crie uma para "Triage Mail" e copie os 16 caracteres.',
							'Fastmail: Acesse Settings -> Privacy & Security -> Integrations -> New app password, marque acesso IMAP e copie a senha.',
							'Clique em "Yahoo" ou "Fastmail" no Preenchimento rápido abaixo, preencha seu e-mail e cole a Senha de aplicativo.'
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
				services:
					'Microsoft 365, Outlook.com, Hotmail e provedores corporativos OIDC compatíveis.',
				redirectTip:
					'O Google bloqueia PKCE público puro sem certificação CASA; para Gmail utilize Senha de Aplicativo ou BYO Credentials.',
				fields: [
					'Preenchimento rápido (Outlook): Preenche automaticamente Host (outlook.office365.com), Porta (993), Endpoint de autorização, Endpoint de token e Escopos IMAP.',
					'ID do cliente (Client ID): O identificador público (UUID) do aplicativo registrado no portal Microsoft Entra / Azure.'
				],
				playbooks: [
					{
						provider: 'Microsoft (Outlook.com / Hotmail / Microsoft 365)',
						steps: [
							'Acesse o portal Microsoft Entra / Azure (portal.azure.com -> Registros de aplicativo / App registrations) e clique em "Novo registro".',
							'Dê o nome "Triage Mail" e, em Tipos de conta com suporte, escolha "Contas em qualquer diretório organizacional e contas pessoais da Microsoft".',
							'Em URI de redirecionamento (Redirect URI), selecione a plataforma "Cliente público/nativo (móvel e área de trabalho)" e digite http://127.0.0.1/callback.',
							'Em Permissões de API -> Adicionar uma permissão -> APIs da Microsoft, adicione as permissões delegadas IMAP.AccessAsUser.All e offline_access.',
							'Na página "Visão geral" (Overview) do aplicativo criado, copie o "ID do aplicativo (cliente)" (Application / client ID).',
							'Clique em "Outlook" no Preenchimento rápido abaixo, informe seu e-mail, cole o ID do cliente e clique em "Autorizar e conectar".'
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
				gcpTip: 'Escopo necessário: https://mail.google.com/ com redirecionamento loopback.',
				fields: [
					'Preenchimento rápido (Gmail OAuth): Preenche automaticamente Host (imap.gmail.com), Porta (993), Endpoints OAuth do Google e o escopo https://mail.google.com/.',
					'ID do cliente e Chave secreta: As credenciais OAuth de aplicativo Desktop geradas no seu projeto do Google Cloud Console.'
				],
				playbooks: [
					{
						provider: 'Google Cloud Console (Gmail / Google Workspace)',
						steps: [
							'Acesse console.cloud.google.com, crie um novo projeto e, em "APIs e Serviços -> Biblioteca", pesquise por "Gmail API" e clique em "Ativar".',
							'Em "APIs e Serviços -> Tela de permissão OAuth", configure como Externo (ou Interno no Workspace), adicione o escopo https://mail.google.com/ e adicione seu próprio e-mail em "Usuários de teste" (Test users).',
							'Vá em "Credenciais -> Criar credenciais -> ID do cliente OAuth" e selecione o tipo de aplicativo "App para computador" (Desktop app).',
							'Copie o "ID do cliente" (Client ID) e a "Chave secreta do cliente" (Client secret) gerados.',
							'Clique em "Gmail OAuth" no Preenchimento rápido abaixo, digite seu e-mail, cole o ID do cliente e a Chave secreta e clique em "Autorizar e conectar".',
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
						provider: 'Proton Mail Bridge (Configuração Local no Computador)',
						steps: [
							'Baixe, instale e abra o aplicativo oficial Proton Mail Bridge no seu computador (requer assinatura Proton paga).',
							'Faça login na sua conta Proton dentro do Bridge e aguarde a sincronização inicial concluir.',
							'Clique sobre sua conta conectada no Bridge e abra "Detalhes da caixa de correio" (Mailbox details / Configuration).',
							'Confirme que a porta IMAP local é 1143 e que o modo de segurança está definido como SSL/TLS ou STARTTLS.',
							'Copie o "Username" e o "Password" mostrados nessa tela do Bridge (nunca utilize a senha da sua conta Proton da web).',
							'Clique em "Proton" no Preenchimento rápido abaixo, cole o usuário e a senha do Bridge e clique em "Conectar conta".'
						]
					}
				]
			}
		}
	}
};
