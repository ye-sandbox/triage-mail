export function safeFilename(filename: string): string {
	// INVARIANT: Sender filenames cannot supply paths, controls, or bidi display overrides.
	const basename = filename.split(/[/\\]/).pop() ?? '';
	return (
		basename
			.replace(/[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069<>:"|?*]/g, '_')
			.replace(/^[.\s]+|[.\s]+$/g, '')
			.slice(0, 180) || 'attachment'
	);
}
