export const ROW_HEIGHT = 88;
export const OVERSCAN = 5;
export function visibleRange(count: number, top: number, height: number, rowHeight = ROW_HEIGHT) {
	const start = Math.min(count, Math.max(0, Math.floor(top / rowHeight) - OVERSCAN));
	const end = Math.min(count, Math.ceil((top + height) / rowHeight) + OVERSCAN);
	return { start, end, offset: start * rowHeight, total: count * rowHeight };
}
export function selectionTop(index: number, top: number, height: number, rowHeight = ROW_HEIGHT) {
	const rowTop = index * rowHeight;
	if (rowTop < top) return rowTop;
	if (rowTop + rowHeight > top + height) return rowTop + rowHeight - height;
	return top;
}
