export const ROW_HEIGHT = 88;
export const OVERSCAN = 5;
export function visibleRange(count: number, top: number, height: number) {
	const start = Math.min(count, Math.max(0, Math.floor(top / ROW_HEIGHT) - OVERSCAN));
	const end = Math.min(count, Math.ceil((top + height) / ROW_HEIGHT) + OVERSCAN);
	return { start, end, offset: start * ROW_HEIGHT, total: count * ROW_HEIGHT };
}
export function selectionTop(index: number, top: number, height: number) {
	const rowTop = index * ROW_HEIGHT;
	if (rowTop < top) return rowTop;
	if (rowTop + ROW_HEIGHT > top + height) return rowTop + ROW_HEIGHT - height;
	return top;
}
