import type { Guide } from '../guides';

/** The 5 GiB screen recording shared by the large-file guides. */
export const largeFileDemo: NonNullable<Guide['video']> = {
	src: '/media/large-file-demo.mp4',
	poster: '/media/large-file-demo-poster.jpg',
	width: 1280,
	height: 800,
	label:
		'Screen recording of TabTool for macOS opening a 5 GiB CSV with 89,158,916 rows, jumping to the last row, fixing one price, and saving.',
	caption: 'Real-time capture · Mac build · 5 GiB · 89,158,916 rows × 8 columns · 2026-11-23'
};
