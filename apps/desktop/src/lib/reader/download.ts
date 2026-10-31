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

export function saveAttachment(bytes: number[], filename: string): () => void {
	// INVARIANT: Treat all downloaded content as a file; never navigate to or render attachment bytes.
	const blob = new Blob([new Uint8Array(bytes)], { type: 'application/octet-stream' });
	const url = URL.createObjectURL(blob);
	const link = document.createElement('a');
	let timer: ReturnType<typeof setTimeout>;
	const release = () => {
		clearTimeout(timer);
		URL.revokeObjectURL(url);
	};
	try {
		link.href = url;
		link.download = safeFilename(filename);
		document.body.append(link);
		link.click();
	} catch (error) {
		release();
		throw error;
	} finally {
		link.remove();
	}
	// Let the webview consume the URL before releasing it; unmount also releases it.
	timer = setTimeout(release, 60_000);
	return release;
}
