import type { SupportedLocale } from '../i18n/types';
import type { EmbeddingState } from './types';

export type SettingsSectionId = 'general' | 'search' | 'ai' | 'privacy' | 'backup';

export interface SettingsLabels {
	pageTitle: string;
	pageSubtitle: string;
	backToInbox: string;
	navAria: string;
	tabs: Record<SettingsSectionId, string>;
	editedBadge: string;
	loading: string;
	loadError: string;
	retryLoading: string;
	unsavedTitle: string;
	unsavedDescription: string;
	stay: string;
	discard: string;
	general: {
		heading: string;
		subtitle: string;
		languageTitle: string;
		languageDesc: string;
		languageAria: string;
		english: string;
		portuguese: string;
		themeTitle: string;
		themeDescPrefix: string;
		detectedScheme: string;
		activeCanvas: string;
		matchOs: (scheme: string) => string;
		previewTheme: (target: string) => string;
		light: string;
		dark: string;
		aboutTitle: string;
		aboutBody: string;
		aboutStack: string;
	};
	privacy: {
		heading: string;
		subtitle: string;
		zeroTelemetryTitle: string;
		zeroTelemetryBody: string;
		zeroTelemetryBadge: string;
		remoteImagesTitle: string;
		remoteImagesBody: string;
		remoteImagesBadge: string;
		htmlSanitizationTitle: string;
		htmlSanitizationBody: string;
		offlineTransportTitle: string;
		offlineTransportBody: string;
		vaultEncryptionTitle: string;
		vaultEncryptionBody: string;
		onDemandAttachmentsTitle: string;
		onDemandAttachmentsBody: string;
	};
	search: {
		heading: string;
		subtitle: string;
		tokenizerTitle: string;
		tokenizerDesc: string;
		tokenizerSelectLabel: string;
		unicodeLabel: string;
		unicodeDesc: string;
		trigramLabel: string;
		trigramDesc: string;
		trigramWarning: string;
		applyTokenizer: string;
		rebuildingTokenizer: string;
		tokenizerUpdated: string;
		tokenizerError: string;
		semanticTitle: string;
		semanticDesc: string;
		semanticToggleAria: string;
		telemetryTitle: string;
		inspectState: string;
		states: Record<EmbeddingState['status'], string>;
		indexed: string;
		pending: string;
		failed: string;
		vectorProgress: string;
		rebuildConfirm: string;
		rebuildBtn: string;
		rebuildingVector: string;
		semanticError: string;
		modelTitle: string;
		modelCached: string;
		modelMissing: string;
		modelInfo: string;
		modelUrlLabel: string;
		tokenizerUrlLabel: string;
		modelPolicy: string;
		modelConsent: string;
		downloadBtn: string;
		downloadingBtn: string;
		downloadOk: string;
		downloadError: string;
	};
	ai: {
		heading: string;
		subtitle: string;
		formAria: string;
		enableAria: string;
		enabledBadge: string;
		disabledBadge: string;
		disclosure: string;
		enableLabel: string;
		providerLabel: string;
		ollamaTitle: string;
		ollamaBadge: string;
		ollamaDesc: string;
		byokTitle: string;
		byokBadge: string;
		byokDesc: string;
		endpointLabel: string;
		endpointHelp: string;
		modelLabel: string;
		apiKeyLabel: string;
		keyStored: string;
		noKeyStored: string;
		keyPlaceholderStored: string;
		keyPlaceholderEmpty: string;
		keyHelp: string;
		saveBtn: string;
		savingBtn: string;
		saveSuccess: string;
		saveError: string;
	};
	backup: {
		heading: string;
		subtitle: string;
		exportTitle: string;
		exportDesc: string;
		restoreTitle: string;
		restoreDesc: string;
		restoreUnavailableTitle: string;
		help: string;
		recovery: string;
		newFile: string;
		passLabel: string;
		confirmLabel: string;
		consentLabel: string;
		exportBtn: string;
		restoreBtn: string;
		continueBtn: string;
		busy: string;
		done: string;
		restored: string;
		error: string;
		mismatch: string;
		portableTitle: string;
		portableDesc: string;
		portableDisclosure: string;
		exportJsonBtn: string;
		importJsonBtn: string;
		previewDiffBtn: string;
		applyImportBtn: string;
		reviewHeading: string;
		mappingHeading: string;
		chooseTarget: string;
		approveImport: string;
		archiveTitle: string;
		archiveDesc: string;
		archiveEmailLabel: string;
		archiveFormatMbox: string;
		archiveFormatEml: string;
		archiveChooseBtn: string;
		archiveNoFile: string;
		archiveImportBtn: string;
		archiveSuccess: string;
		archiveConflict: string;
		archiveError: string;
	};
	importDialog: {
		title: string;
		closeAria: string;
		added: string;
		updated: string;
		conflicting: string;
		keepLocal: string;
		overwrite: string;
		cancel: string;
		commit: string;
		rulesCount: string;
		viewsCount: string;
		noItems: string;
	};
}

export const settingsLabels: Record<SupportedLocale, SettingsLabels> = {
	en: {
		pageTitle: 'Settings, Search Index, Local AI & Backup',
		pageSubtitle:
			'Local-first indexing, ONNX artifacts, vault-sealed AI credentials, and portable configuration snapshots.',
		backToInbox: 'Back to inbox',
		navAria: 'Settings categories',
		tabs: {
			general: 'General',
			search: 'Search & Embeddings',
			ai: 'AI & Local Models',
			privacy: 'Privacy & Network',
			backup: 'Backup & Portable Config'
		},
		editedBadge: 'Edited',
		loading: 'Loading settings…',
		loadError: 'Settings could not be loaded.',
		retryLoading: 'Retry loading settings',
		unsavedTitle: 'You have unsaved changes',
		unsavedDescription:
			'You have uncommitted edits in this settings section. Stay on form to review and save them, or discard changes before leaving.',
		stay: 'Stay on form',
		discard: 'Discard changes',
		general: {
			heading: 'General Preferences',
			subtitle: 'Interface language, operating-system appearance alignment, and application details.',
			languageTitle: 'Interface language',
			languageDesc:
				'Applies localized labels, Brazilian currency formatting, and timestamp conventions across all panes.',
			languageAria: 'Interface language',
			english: 'English',
			portuguese: 'Português (Brasil)',
			themeTitle: 'Theme Alignment',
			themeDescPrefix: 'Surface polarity follows your OS prefers-color-scheme preference.',
			detectedScheme: 'Detected system scheme',
			activeCanvas: 'Active canvas',
			matchOs: (scheme) => `Match OS (${scheme})`,
			previewTheme: (target) => `Preview ${target}`,
			light: 'Light',
			dark: 'Dark',
			aboutTitle: 'About triage-mail',
			aboutBody:
				'100% Local-First, Provider-Agnostic, Open-Source (AGPLv3) Email Aggregator & Triage Engine.',
			aboutStack: 'Zero Electron, zero Python/Node sidecars, zero telemetry.'
		},
		privacy: {
			heading: 'Privacy & Network Guarantees',
			subtitle: 'Zero-telemetry principles, remote image blocking, and strict offline-first network rules.',
			zeroTelemetryTitle: 'Zero Telemetry & Network Isolation',
			zeroTelemetryBody:
				'No analytics SDKs, crash reporters, behavioral beacons, or background telemetry pings. Outbound connections occur strictly to your configured IMAP servers, explicit RFC 8058 unsubscribe clicks, or explicitly enabled AI endpoints.',
			zeroTelemetryBadge: 'Verified · 0 outbound beacons',
			remoteImagesTitle: 'Remote Images & Tracking Pixels',
			remoteImagesBody:
				'Remote images remain blocked by default to prevent tracking pixels from reporting your IP or read status. You can allow HTTPS images for a single open message in the reader; changing messages resets consent immediately.',
			remoteImagesBadge: 'Blocked by default',
			htmlSanitizationTitle: 'HTML Sanitization',
			htmlSanitizationBody:
				'Strips scripts, forms, iframes, and inline SVG payloads before rendering.',
			offlineTransportTitle: 'Offline-First Transport',
			offlineTransportBody:
				'No HTTP redirects and no ambient proxy traversal for local or pinned downloads.',
			vaultEncryptionTitle: 'Local Vault & Database Encryption',
			vaultEncryptionBody:
				'Credentials and OAuth tokens stay in OS Keyring via Argon2id; mail data uses SQLCipher.',
			onDemandAttachmentsTitle: 'On-Demand Attachments',
			onDemandAttachmentsBody: 'IMAP BODY.PEEK fetches attachments only when explicitly clicked.'
		},
		search: {
			heading: 'Search & Local Embeddings',
			subtitle:
				'SQLite FTS5 lexical tokenization, experimental sqlite-vec int8 KNN search, and pinned ONNX cache.',
			tokenizerTitle: 'FTS5 Lexical Tokenizer',
			tokenizerDesc:
				'Controls how subjects, senders, tags, and extracted codes are tokenized inside SQLite.',
			tokenizerSelectLabel: 'Tokenizer',
			unicodeLabel: 'Unicode words (ignore accents)',
			unicodeDesc: 'Fold diacritics across Portuguese and English prose with compact FTS5 storage.',
			trigramLabel: 'Trigram (substring matching)',
			trigramDesc: 'Sub-word indexing for invoice keys, Boletos, and partial domain matches.',
			trigramWarning:
				'Disk-space warning: Trigram needs at least three characters for substring matching and increases SQLite FTS5 storage footprint by ~2.5×–3×.',
			applyTokenizer: 'Apply & Rebuild Index',
			rebuildingTokenizer: 'Rebuilding…',
			tokenizerUpdated: 'Search index updated.',
			tokenizerError: 'Search index could not be updated.',
			semanticTitle: 'Local semantic search',
			semanticDesc:
				'Neural MiniLM embeddings run on your computer. Enabling starts indexing cached text; no messages are sent and no files are downloaded automatically.',
			semanticToggleAria: 'Enable semantic search',
			telemetryTitle: 'Embedding Pipeline Telemetry',
			inspectState: 'Inspect state:',
			states: {
				Disabled: 'Disabled',
				MissingModelOrRuntime:
					'Install the verified model and CPU runtime. See the setup guide.',
				Rebuilding: 'Indexing cached text…',
				Failed: 'Indexing failed. Rebuild to retry.',
				Ready: 'Ready',
				Unavailable: 'Status unavailable'
			},
			indexed: 'Indexed',
			pending: 'Pending',
			failed: 'Failed',
			vectorProgress: 'Vector progress (384d int8 KNN)',
			rebuildConfirm:
				'I confirm rebuilding. Semantic search will be unavailable until it finishes.',
			rebuildBtn: 'Rebuild local index',
			rebuildingVector: 'Rebuilding…',
			semanticError: 'The local index could not be updated.',
			modelTitle: 'Local model files',
			modelCached: 'Files verified',
			modelMissing: 'Files missing or invalid',
			modelInfo:
				'Neural MiniLM on CPU. Requires the separately installed verified ONNX runtime. Enable indexing in search settings.',
			modelUrlLabel: 'Direct ONNX model HTTPS URL',
			tokenizerUrlLabel: 'Direct tokenizer.json HTTPS URL',
			modelPolicy:
				'Only the pinned MiniLM revision in the guide is accepted. Limits: 128 MiB model, 8 MiB tokenizer. No redirects or URL credentials. Each file has a two-minute deadline. Hosts receive your IP, never your messages.',
			modelConsent: 'I consent to download both files from these URLs.',
			downloadBtn: 'Download model files',
			downloadingBtn: 'Downloading…',
			downloadOk: 'Model files verified.',
			downloadError:
				'Download or verification failed. Check direct URLs and supported revision files.'
		},
		ai: {
			heading: 'AI Summarization & Local Models',
			subtitle: 'Optional on-demand thread summaries via local Ollama loopback or BYOK HTTPS endpoint.',
			formAria: 'AI settings',
			enableAria: 'Enable summary requests',
			enabledBadge: 'Enabled',
			disabledBadge: 'Disabled',
			disclosure:
				'Disabled by default. Each summary needs your confirmation before sending the subject and plain-text body (up to 8,000 characters) to your chosen endpoint. Cloud providers may charge you.',
			enableLabel: 'Enable summary requests',
			providerLabel: 'Provider',
			ollamaTitle: 'Ollama (Local Loopback)',
			ollamaBadge: 'Offline',
			ollamaDesc: 'http://127.0.0.1:11434',
			byokTitle: 'BYOK (OpenAI compatible)',
			byokBadge: 'TLS · Explicit click',
			byokDesc: 'HTTPS endpoint + vault key',
			endpointLabel: 'Endpoint URL',
			endpointHelp:
				'Ollama requires loopback HTTP/HTTPS. BYOK requires a public HTTPS domain. No embedded credentials, query strings or fragments.',
			modelLabel: 'Model',
			apiKeyLabel: 'API key',
			keyStored: 'A key is stored in your vault.',
			noKeyStored: 'No API key is stored.',
			keyPlaceholderStored: 'Key stored in vault — leave blank to keep for same endpoint',
			keyPlaceholderEmpty: 'Enter API key (never displayed or returned from vault)',
			keyHelp:
				'Leave blank to retain a key for the same endpoint. Switching endpoints requires a new key. Saving Ollama clears the key.',
			saveBtn: 'Save AI settings',
			savingBtn: 'Saving…',
			saveSuccess: 'AI settings saved. No request was sent.',
			saveError: 'AI settings could not be saved. Check the endpoint, model and API key.'
		},
		backup: {
			heading: 'Backup & portable definitions',
			subtitle: 'Full SQLCipher encrypted archives (.triagebak) and secret-free portable JSON definitions.',
			exportTitle: 'Create encrypted backup',
			exportDesc:
				'Seals cached mail, headers, rules, saved views, and sender choices with Argon2id + AES-256-GCM.',
			restoreTitle: 'Restore encrypted backup',
			restoreDesc: 'Verifies archive integrity into an isolated fresh local vault.',
			restoreUnavailableTitle: 'Restore requires a fresh installation',
			help: 'Backups include cached mail, headers, attachment metadata, mailbox state, rules, saved views, local categories, sender choices, dismissals and account photos. Unfetched attachment binaries, external attachment files, models, credentials, network settings, pending actions and delivery history are absent. Mail itself can contain private data.',
			recovery:
				'Restore is available before first setup into a fresh installation only. Existing data and vaults are never replaced. Recovered accounts stay offline; reconnect by adding the same email address in Accounts. Rules, reminders and destinations stay disabled. AI permissions reset. No pending actions replay.',
			newFile:
				'Choose a new filename; existing files are never overwritten. Native file selection requires a Linux desktop portal.',
			passLabel: 'Backup passphrase (at least 12 characters)',
			confirmLabel: 'Confirm passphrase',
			consentLabel:
				'I understand the included data and omissions. For restore, I will unlock the new isolated local vault with this backup passphrase. A lost passphrase cannot be recovered.',
			exportBtn: 'Create encrypted backup',
			restoreBtn: 'Restore encrypted backup',
			continueBtn: 'Continue setup',
			busy: 'Working…',
			done: 'Completed.',
			restored:
				'Backup restored. Continue and unlock the new local vault using the backup passphrase.',
			error:
				'Operation failed. Check the passphrase, format, file access and available disk space. Choose a new output filename. Recovery requires a fresh installation.',
			mismatch: 'Passphrases must match and contain at least 12 characters.',
			portableTitle: 'Portable Definitions (Rules, Saved Views, Categories)',
			portableDesc:
				'Human-readable JSON without credentials or message bodies. Imports open a review before committing.',
			portableDisclosure:
				'Portable JSON is readable and may contain private names, queries and predicates. It excludes credentials and replaces account/destination IDs with placeholders. Review every definition and map every reference. Rules import disabled with fresh IDs; existing definitions stay intact. All-account definitions continue to cover all accounts.',
			exportJsonBtn: 'Export rules & saved views',
			importJsonBtn: 'Choose definitions to import',
			previewDiffBtn: 'Preview Diff',
			applyImportBtn: 'Import reviewed definitions',
			reviewHeading: 'Review definitions',
			mappingHeading: 'Map references',
			chooseTarget: 'Select a target',
			approveImport:
				'I reviewed these definitions, mappings and all-account scopes. Import without running rules.',
			archiveTitle: 'Local Archive Import (.mbox / .eml)',
			archiveDesc:
				'Stream local .mbox exports (Google Takeout, Thunderbird) or single .eml files directly into encrypted SQLCipher storage without IMAP credentials.',
			archiveEmailLabel: 'Archive label or email address',
			archiveFormatMbox: '.mbox archive',
			archiveFormatEml: '.eml message',
			archiveChooseBtn: 'Choose .mbox / .eml file',
			archiveNoFile: 'No archive file selected',
			archiveImportBtn: 'Import local archive',
			archiveSuccess: 'Imported archive messages:',
			archiveConflict:
				'Email belongs to an existing IMAP or recovered account. Use a distinct address.',
			archiveError: 'Archive import failed. Verify the file format and permissions.'
		},
		importDialog: {
			title: 'Portable Definitions Import Preview',
			closeAria: 'Close preview',
			added: 'Added',
			updated: 'Updated',
			conflicting: 'Conflicting',
			keepLocal: 'Keep Local',
			overwrite: 'Overwrite',
			cancel: 'Cancel',
			commit: 'Commit Import',
			rulesCount: 'Rules',
			viewsCount: 'Saved views',
			noItems: 'No differences detected in this preview.'
		}
	},
	'pt-BR': {
		pageTitle: 'Configurações, Índice de Busca, IA Local e Backup',
		pageSubtitle:
			'Indexação local, artefatos ONNX, credenciais de IA protegidas no cofre e cópias portáteis.',
		backToInbox: 'Voltar à caixa de entrada',
		navAria: 'Categorias de configurações',
		tabs: {
			general: 'Geral',
			search: 'Busca e Embeddings',
			ai: 'IA e Modelos Locais',
			privacy: 'Privacidade e Rede',
			backup: 'Backup e Definições'
		},
		editedBadge: 'Editado',
		loading: 'Carregando configurações…',
		loadError: 'Não foi possível carregar as configurações.',
		retryLoading: 'Tentar carregar novamente',
		unsavedTitle: 'Você tem alterações não salvas',
		unsavedDescription:
			'Há edições pendentes nesta seção. Permanecer no formulário para revisar e salvar ou descartar alterações antes de sair.',
		stay: 'Permanecer no formulário',
		discard: 'Descartar alterações',
		general: {
			heading: 'Preferências Gerais',
			subtitle: 'Idioma da interface, alinhamento com o tema do sistema e informações do aplicativo.',
			languageTitle: 'Idioma da interface',
			languageDesc:
				'Aplica rótulos localizados, formatação monetária em BRL e convenções de data em todos os painéis.',
			languageAria: 'Idioma da interface',
			english: 'English',
			portuguese: 'Português (Brasil)',
			themeTitle: 'Alinhamento de Tema',
			themeDescPrefix: 'A polaridade segue a preferência prefers-color-scheme do sistema.',
			detectedScheme: 'Esquema detectado',
			activeCanvas: 'Tema ativo',
			matchOs: (scheme) => `Seguir sistema (${scheme})`,
			previewTheme: (target) => `Visualizar ${target}`,
			light: 'Claro',
			dark: 'Escuro',
			aboutTitle: 'Sobre o triage-mail',
			aboutBody:
				'Agregador e motor de triagem de e-mail 100% local, independente de provedor e código aberto (AGPLv3).',
			aboutStack: 'Zero Electron, zero sidecars Python/Node, zero telemetria.'
		},
		privacy: {
			heading: 'Garantias de Privacidade e Rede',
			subtitle:
				'Princípios de zero telemetria, bloqueio de imagens remotas e regras rígidas offline-first.',
			zeroTelemetryTitle: 'Zero Telemetria e Isolamento de Rede',
			zeroTelemetryBody:
				'Sem SDKs de análise, relatórios de falha automáticos ou rastreadores. Conexões de saída ocorrem apenas para seus servidores IMAP, cliques explícitos em RFC 8058 ou endpoints de IA ativados.',
			zeroTelemetryBadge: 'Verificado · 0 beacons externos',
			remoteImagesTitle: 'Imagens Remotas e Pixels de Rastreamento',
			remoteImagesBody:
				'Imagens remotas permanecem bloqueadas por padrão para evitar vazamento de IP e confirmação de leitura. Você pode permitir HTTPS para uma única mensagem aberta no leitor; trocar de mensagem revoga o consentimento.',
			remoteImagesBadge: 'Bloqueadas por padrão',
			htmlSanitizationTitle: 'Sanitização de HTML',
			htmlSanitizationBody:
				'Remove scripts, formulários, iframes e SVGs inline antes da renderização.',
			offlineTransportTitle: 'Transporte Offline-First',
			offlineTransportBody:
				'Sem redirecionamentos HTTP nem proxies automáticos em downloads locais ou fixados.',
			vaultEncryptionTitle: 'Cofre Local e Banco Criptografado',
			vaultEncryptionBody:
				'Credenciais e tokens OAuth ficam no chaveiro do sistema com Argon2id; dados usam SQLCipher.',
			onDemandAttachmentsTitle: 'Anexos Sob Demanda',
			onDemandAttachmentsBody: 'IMAP BODY.PEEK baixa anexos apenas quando clicados explicitamente.'
		},
		search: {
			heading: 'Busca e Embeddings Locais',
			subtitle:
				'Tokenização lexical SQLite FTS5, busca vetorial experimental sqlite-vec int8 e cache ONNX fixado.',
			tokenizerTitle: 'Tokenizador Lexical FTS5',
			tokenizerDesc:
				'Controla como assuntos, remetentes, tags e códigos extraídos são indexados no SQLite.',
			tokenizerSelectLabel: 'Tokenizador',
			unicodeLabel: 'Palavras Unicode (ignorar acentos)',
			unicodeDesc: 'Normaliza acentos em português e inglês com menor uso de disco.',
			trigramLabel: 'Trigram (correspondência por substring)',
			trigramDesc: 'Indexação por subpalavras para chaves fiscais, boletos e trechos parciais.',
			trigramWarning:
				'Aviso de espaço em disco: Trigram exige ao menos três caracteres para substrings e aumenta o índice FTS5 em ~2,5×–3×.',
			applyTokenizer: 'Aplicar e Reconstruir Índice',
			rebuildingTokenizer: 'Reconstruindo…',
			tokenizerUpdated: 'Índice de busca atualizado.',
			tokenizerError: 'Não foi possível atualizar o índice de busca.',
			semanticTitle: 'Busca semântica local',
			semanticDesc:
				'Embeddings neurais MiniLM no seu computador. Ativar inicia a indexação do texto em cache; nenhuma mensagem é enviada e nenhum arquivo é baixado automaticamente.',
			semanticToggleAria: 'Ativar busca semântica',
			telemetryTitle: 'Telemetria do Pipeline de Embeddings',
			inspectState: 'Inspecionar estado:',
			states: {
				Disabled: 'Desativada',
				MissingModelOrRuntime:
					'Instale o modelo verificado e o runtime CPU. Consulte o guia de configuração.',
				Rebuilding: 'Indexando texto em cache…',
				Failed: 'Falha na indexação. Reconstrua para tentar novamente.',
				Ready: 'Pronta',
				Unavailable: 'Estado indisponível'
			},
			indexed: 'Indexadas',
			pending: 'Pendentes',
			failed: 'Falhas',
			vectorProgress: 'Progresso vetorial (384d int8 KNN)',
			rebuildConfirm:
				'Confirmo a reconstrução. A busca semântica ficará indisponível até a conclusão.',
			rebuildBtn: 'Reconstruir índice local',
			rebuildingVector: 'Reconstruindo…',
			semanticError: 'Não foi possível atualizar o índice local.',
			modelTitle: 'Arquivos do modelo local',
			modelCached: 'Arquivos verificados',
			modelMissing: 'Arquivos ausentes ou inválidos',
			modelInfo:
				'MiniLM neural em CPU. Requer o runtime ONNX verificado, instalado separadamente. A busca precisa ser ativada nas configurações de busca.',
			modelUrlLabel: 'URL HTTPS direta do modelo ONNX',
			tokenizerUrlLabel: 'URL HTTPS direta do tokenizer.json',
			modelPolicy:
				'Somente a revisão MiniLM fixada no guia é aceita. Limites: modelo 128 MiB, tokenizer 8 MiB. Sem redirecionamentos ou credenciais na URL. Cada arquivo tem prazo de dois minutos. Os servidores recebem seu IP, nunca suas mensagens.',
			modelConsent: 'Autorizo baixar os dois arquivos dessas URLs.',
			downloadBtn: 'Baixar arquivos do modelo',
			downloadingBtn: 'Baixando…',
			downloadOk: 'Arquivos do modelo verificados.',
			downloadError:
				'Falha no download ou na verificação. Confira as URLs diretas e os arquivos da revisão suportada.'
		},
		ai: {
			heading: 'Resumos com IA e Modelos Locais',
			subtitle:
				'Resumos opcionais sob demanda via Ollama local ou endpoint HTTPS BYOK.',
			formAria: 'Configurações de IA',
			enableAria: 'Ativar solicitações de resumo',
			enabledBadge: 'Ativada',
			disabledBadge: 'Desativada',
			disclosure:
				'Desativada por padrão. Cada resumo requer sua confirmação antes de enviar o assunto e o texto simples (até 8.000 caracteres) ao endpoint escolhido. Provedores em nuvem podem cobrar.',
			enableLabel: 'Ativar solicitações de resumo',
			providerLabel: 'Provedor',
			ollamaTitle: 'Ollama (Loopback Local)',
			ollamaBadge: 'Offline',
			ollamaDesc: 'http://127.0.0.1:11434',
			byokTitle: 'BYOK (Compatível OpenAI)',
			byokBadge: 'TLS · Clique explícito',
			byokDesc: 'Endpoint HTTPS + chave no cofre',
			endpointLabel: 'URL do Endpoint',
			endpointHelp:
				'Ollama exige HTTP/HTTPS em loopback. BYOK exige domínio HTTPS público. Sem credenciais na URL, query strings ou fragmentos.',
			modelLabel: 'Modelo',
			apiKeyLabel: 'Chave de API',
			keyStored: 'Uma chave está armazenada no seu cofre.',
			noKeyStored: 'Nenhuma chave de API armazenada.',
			keyPlaceholderStored: 'Chave no cofre — deixe em branco para manter no mesmo endpoint',
			keyPlaceholderEmpty: 'Informe a chave de API (nunca exibida ou retornada do cofre)',
			keyHelp:
				'Deixe em branco para manter a chave no mesmo endpoint. Trocar de endpoint exige nova chave. Salvar Ollama limpa a chave.',
			saveBtn: 'Salvar configurações de IA',
			savingBtn: 'Salvando…',
			saveSuccess: 'Configurações de IA salvas. Nenhuma requisição foi enviada.',
			saveError:
				'Não foi possível salvar as configurações de IA. Verifique o endpoint, o modelo e a chave.'
		},
		backup: {
			heading: 'Backup e definições portáteis',
			subtitle: 'Arquivos SQLCipher criptografados (.triagebak) e definições JSON sem segredos.',
			exportTitle: 'Criar backup criptografado',
			exportDesc:
				'Protege mensagens em cache, regras, buscas salvas e decisões com Argon2id + AES-256-GCM.',
			restoreTitle: 'Restaurar backup criptografado',
			restoreDesc: 'Verifica a integridade do arquivo em um novo cofre local isolado.',
			restoreUnavailableTitle: 'A restauração exige uma instalação nova',
			help: 'O backup inclui mensagens em cache, cabeçalhos, metadados de anexos, estado das pastas, regras, buscas salvas, categorias locais, escolhas de remetentes, dispensas e fotos de contas. Não inclui binários de anexos não baixados, arquivos externos de anexos, modelos, credenciais, configurações de rede, ações pendentes ou histórico de envios. As mensagens podem conter dados privados.',
			recovery:
				'A restauração só está disponível antes da configuração inicial, em uma instalação nova. Dados e cofres existentes nunca são substituídos. As contas restauradas ficam offline; reconecte adicionando o mesmo endereço em Contas. Regras, lembretes e destinos ficam desativados. Permissões de IA são redefinidas. Ações pendentes não são repetidas.',
			newFile:
				'Escolha um nome novo; arquivos existentes nunca são substituídos. A seleção nativa exige um portal de desktop Linux.',
			passLabel: 'Senha do backup (pelo menos 12 caracteres)',
			confirmLabel: 'Confirme a senha',
			consentLabel:
				'Entendo os dados incluídos e as omissões. Na restauração, desbloquearei o novo cofre local isolado com esta senha do backup. Uma senha perdida não pode ser recuperada.',
			exportBtn: 'Criar backup criptografado',
			restoreBtn: 'Restaurar backup criptografado',
			continueBtn: 'Continuar configuração',
			busy: 'Processando…',
			done: 'Concluído.',
			restored:
				'Backup restaurado. Continue e desbloqueie o novo cofre local usando a senha do backup.',
			error:
				'Falha na operação. Verifique a senha, o formato, o acesso ao arquivo e o espaço em disco. Escolha um novo nome de saída. A restauração exige uma instalação nova.',
			mismatch: 'As senhas devem ser iguais e ter pelo menos 12 caracteres.',
			portableTitle: 'Definições Portáteis (Regras, Buscas Salvas, Categorias)',
			portableDesc:
				'JSON legível sem credenciais ou corpos de mensagens. A importação abre revisão antes de aplicar.',
			portableDisclosure:
				'O JSON portátil é legível e pode conter nomes, consultas e condições privadas. Exclui credenciais e substitui IDs de contas e destinos por referências. Revise cada definição e mapeie todas as referências. As regras são importadas desativadas com novos IDs; as existentes são preservadas. Definições para todas as contas continuam abrangendo todas as contas.',
			exportJsonBtn: 'Exportar regras e buscas salvas',
			importJsonBtn: 'Escolher definições para importar',
			previewDiffBtn: 'Visualizar Diferenças',
			applyImportBtn: 'Importar definições revisadas',
			reviewHeading: 'Revisar definições',
			mappingHeading: 'Mapear referências',
			chooseTarget: 'Selecione um destino',
			approveImport:
				'Revisei as definições, os mapeamentos e os escopos de todas as contas. Importar sem executar regras.',
			archiveTitle: 'Importação de Arquivo Local (.mbox / .eml)',
			archiveDesc:
				'Importe arquivos .mbox locais (Google Takeout, Thunderbird) ou mensagens .eml diretamente para o SQLCipher criptografado sem credenciais IMAP.',
			archiveEmailLabel: 'Rótulo ou e-mail do arquivo',
			archiveFormatMbox: 'Arquivo .mbox',
			archiveFormatEml: 'Mensagem .eml',
			archiveChooseBtn: 'Escolher arquivo .mbox / .eml',
			archiveNoFile: 'Nenhum arquivo selecionado',
			archiveImportBtn: 'Importar arquivo local',
			archiveSuccess: 'Mensagens importadas do arquivo:',
			archiveConflict:
				'E-mail pertence a uma conta IMAP ou recuperada. Use outro endereço.',
			archiveError: 'Falha na importação do arquivo. Verifique o formato e as permissões.'
		},
		importDialog: {
			title: 'Pré-visualização de Importação Portátil',
			closeAria: 'Fechar pré-visualização',
			added: 'Adicionadas',
			updated: 'Atualizadas',
			conflicting: 'Conflitantes',
			keepLocal: 'Manter local',
			overwrite: 'Sobrescrever',
			cancel: 'Cancelar',
			commit: 'Confirmar importação',
			rulesCount: 'Regras',
			viewsCount: 'Buscas salvas',
			noItems: 'Nenhuma diferença detectada nesta pré-visualização.'
		}
	}
};
