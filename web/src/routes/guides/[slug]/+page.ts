import { error } from '@sveltejs/kit';
import { guides, guideBySlug } from '#lib/guides';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () => guides.map((item) => ({ slug: item.slug }));

export const load: PageLoad = ({ params }) => {
	const guide = guideBySlug(params.slug);
	if (!guide) error(404, 'Guide not found');
	return { guide };
};
