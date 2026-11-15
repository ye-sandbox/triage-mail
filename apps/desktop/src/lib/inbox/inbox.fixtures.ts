import type { EmailMessage } from '$lib/types';

export const INBOX_FIXTURES: EmailMessage[] = [
	{
		id: 'msg_01_otp',
		thread_id: 'thr_auth_492810',
		account_id: 'acct_personal_02',
		folder: 'INBOX',
		sender_name: 'Bank Auth Security',
		sender_email: 'security@bank-auth.example.com',
		sender_status: 'Approved',
		subject: 'Your verification code is 492810',
		snippet: 'Use one-time verification code 492810 to authorize your new local device session.',
		received_at_iso: '2026-11-14T14:31:00-08:00',
		unread: true,
		starred: false,
		snoozed_until_iso: null,
		category: 'Security',
		tags: ['2FA'],
		extractions: {
			otp: {
				code: '492810',
				issuer: 'Bank Auth'
			}
		},
		attachments_meta: [],
		html_raw: `<div>
			<p>Hello,</p>
			<p>Enter the one-time security code below to complete your sign-in:</p>
			<p><strong>492810</strong></p>
			<p>This code expires in 10 minutes. If you did not request this login, lock your vault immediately.</p>
		</div>`,
		text_fallback: 'Your verification code is 492810. Expires in 10 minutes.'
	},
	{
		id: 'msg_02_boleto',
		thread_id: 'thr_energia_nov',
		account_id: 'acct_personal_02',
		folder: 'INBOX',
		sender_name: 'Energia Sul Distribuidora',
		sender_email: 'billing@energia.example.org',
		sender_status: 'Approved',
		subject: 'Fatura Mensal - Vencimento 20/11',
		snippet: 'Sua fatura de energia elétrica referente a novembro/2026 está disponível para pagamento.',
		received_at_iso: '2026-11-14T12:18:00-08:00',
		unread: true,
		starred: false,
		snoozed_until_iso: null,
		category: 'Financial',
		tags: ['Utilities'],
		extractions: {
			boleto: {
				amount_cents: 28450,
				due_date_iso: '2026-11-20',
				digitable_line: '34191.79001 01043.510047 91020.150008 8 99120000028450',
				beneficiary: 'Energia Sul S.A.',
				paid: false
			}
		},
		attachments_meta: [
			{
				id: 'att_fatura_nov',
				filename: 'fatura-nov.pdf',
				mime_type: 'application/pdf',
				size_bytes: 142840,
				downloaded: false
			}
		],
		html_raw: `<div>
			<h3>Fatura Mensal — Energia Sul</h3>
			<p>Prezado(a) cliente, segue o resumo da sua conta mensal:</p>
			<ul>
				<li><strong>Valor:</strong> R$ 284,50</li>
				<li><strong>Vencimento:</strong> 20/11/2026</li>
				<li><strong>Linha digitável:</strong> <code>34191.79001 01043.510047 91020.150008 8 99120000028450</code></li>
			</ul>
			<p>O PDF completo está anexado como <strong>fatura-nov.pdf</strong> para download sob demanda.</p>
		</div>`,
		text_fallback:
			'Fatura Mensal Energia Sul. Valor R$ 284,50. Vencimento 20/11/2026. Linha digitável: 34191.79001 01043.510047 91020.150008 8 99120000028450'
	},
	{
		id: 'msg_03_shopping',
		thread_id: 'thr_loja_8821',
		account_id: 'acct_work_01',
		folder: 'INBOX',
		sender_name: 'Loja Central',
		sender_email: 'orders@loja.example.com',
		sender_status: 'Pending',
		subject: 'Pedido enviado + Nota Fiscal',
		snippet: 'Seu pedido #8821 foi despachado via Correios (BR123456789BR) com NF-e emitida.',
		received_at_iso: '2026-11-14T11:15:00-08:00',
		unread: true,
		starred: false,
		snoozed_until_iso: null,
		category: 'Shopping',
		tags: ['Hardware'],
		extractions: {
			tracking: {
				carrier: 'Correios',
				code: 'BR123456789BR',
				status_hint: 'Objeto postado'
			},
			nfe: {
				access_key: '35261112345678000195550010000482911000918273',
				invoice_number: '000.048.291',
				issuer: 'Loja Central Comércio Ltda',
				amount_cents: 64900
			}
		},
		attachments_meta: [
			{
				id: 'att_nfe_xml',
				filename: 'nfe-000048291.xml',
				mime_type: 'application/xml',
				size_bytes: 18420,
				downloaded: false
			}
		],
		html_raw: `<div>
			<p>Olá! Seu pedido <strong>#8821</strong> foi coletado pela transportadora.</p>
			<p><strong>Código de rastreio (Correios):</strong> <code>BR123456789BR</code></p>
			<p><strong>Chave de acesso NF-e:</strong> <code>35261112345678000195550010000482911000918273</code></p>
			<blockquote>Nota Fiscal Eletrônica disponível em anexo XML para arquivamento fiscal.</blockquote>
		</div>`,
		text_fallback:
			'Pedido enviado via Correios BR123456789BR. Chave NF-e: 35261112345678000195550010000482911000918273'
	},
	{
		id: 'msg_04_newsletter',
		thread_id: 'thr_rustweekly_142',
		account_id: 'acct_work_01',
		folder: 'INBOX',
		sender_name: 'Systems Engineering Weekly',
		sender_email: 'digest@rustweekly.example.org',
		sender_status: 'Approved',
		subject: 'Systems Engineering Weekly #142',
		snippet: 'Zero-copy IMAP parsers, SQLite WAL compaction strategies, and deterministic local queues.',
		received_at_iso: '2026-11-14T08:40:00-08:00',
		unread: false,
		starred: false,
		snoozed_until_iso: null,
		category: 'Newsletter',
		tags: ['Reading'],
		extractions: {
			unsubscribe: {
				supports_one_click: true,
				endpoint_hint: 'https://lists.rustweekly.example.org/unsubscribe/142',
				unsubscribed: false
			}
		},
		attachments_meta: [],
		html_raw: `<div>
			<h2>Systems Engineering Weekly — Issue #142</h2>
			<p>Highlights from this week's systems architecture reading list:</p>
			<img src="https://cdn.rustweekly.example.org/issues/142/diagram.png" alt="SQLite WAL architecture diagram" width="560" height="220" />
			<ol>
				<li><strong>Zero-Copy IMAP Parsers:</strong> Avoiding intermediate heap allocations during envelope scans.</li>
				<li><strong>SQLite FTS5 + Local Embeddings:</strong> Hybrid lexical ranking with deterministic tie-breakers.</li>
				<li><strong>RFC 8058 One-Click Unsubscribe:</strong> Clean list-unsubscribe headers without browser redirects.</li>
			</ol>
			<img src="https://track.rustweekly.example.org/open/pixel.gif" alt="" width="1" height="1" />
			<p>Read the archived plain-text edition at <a href="https://rustweekly.example.org/142">rustweekly.example.org/142</a>.</p>
		</div>`,
		text_fallback:
			'Systems Engineering Weekly #142: Zero-copy IMAP parsers, SQLite FTS5 hybrid ranking, and RFC 8058 One-Click Unsubscribe.'
	},
	{
		id: 'msg_05_pix',
		thread_id: 'thr_studio_invoice',
		account_id: 'acct_work_01',
		folder: 'INBOX',
		sender_name: 'Alex Rivera',
		sender_email: 'alex@studio.example.com',
		sender_status: 'Approved',
		subject: 'Re: Shared studio invoice',
		snippet: 'Hey, splitting the November fiber + desk studio bill here. Pix Copia e Cola below.',
		received_at_iso: '2026-11-13T17:05:00-08:00',
		unread: false,
		starred: true,
		snoozed_until_iso: null,
		category: 'Personal',
		tags: ['Studio', 'Urgent'],
		extractions: {
			pix: {
				payload:
					'00020126580014br.gov.bcb.pix0136a1b2c3d4-0000-4000-8000-0000000000015204000053039865406420.005802BR5911Alex Rivera6009Sao Paulo62070503***6304A1B2',
				amount_cents: 42000,
				recipient: 'Alex Rivera'
			}
		},
		attachments_meta: [
			{
				id: 'att_studio_split',
				filename: 'studio-nov-split.csv',
				mime_type: 'text/csv',
				size_bytes: 3120,
				downloaded: false
			}
		],
		html_raw: `<div>
			<p>Hey,</p>
			<p>Here is the split for November's shared studio fiber and desk utilities (<strong>R$ 420,00</strong>).</p>
			<blockquote>On Nov 13, you wrote: Send over the Pix payload whenever the invoice closes.</blockquote>
			<p>Pix Copia e Cola:</p>
			<pre>00020126580014br.gov.bcb.pix0136a1b2c3d4-0000-4000-8000-0000000000015204000053039865406420.005802BR5911Alex Rivera6009Sao Paulo62070503***6304A1B2</pre>
		</div>`,
		text_fallback: 'Shared studio invoice split R$ 420,00. Pix Copia e Cola included.'
	}
];
