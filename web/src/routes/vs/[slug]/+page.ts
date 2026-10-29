import { error } from '@sveltejs/kit';
import { comparisons, comparisonBySlug } from '#lib/comparisons';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () => comparisons.map((item) => ({ slug: item.slug }));

export const load: PageLoad = ({ params }) => {
	const comparison = comparisonBySlug(params.slug);
	if (!comparison) error(404, 'Comparison not found');
	return { comparison };
};
