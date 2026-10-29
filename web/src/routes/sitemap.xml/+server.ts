import { comparisons } from '#lib/comparisons';
import { site } from '#lib/site';

export const prerender = true;

export function GET() {
	const paths = [
		'/',
		'/download/',
		'/faq/',
		'/feedback/',
		'/vs/',
		...comparisons.map((item) => `/vs/${item.slug}/`),
		'/privacy/',
		'/terms/'
	];

	const urls = paths
		.map(
			(path) =>
				`	<url><loc>${site.url}${path}</loc><lastmod>${site.updated}</lastmod><changefreq>${path === '/' ? 'monthly' : 'yearly'}</changefreq><priority>${path === '/' ? '1.0' : path.startsWith('/vs/') ? '0.7' : '0.5'}</priority></url>`
		)
		.join('\n');

	const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>`;

	return new Response(body, {
		headers: { 'Content-Type': 'application/xml; charset=utf-8' }
	});
}
