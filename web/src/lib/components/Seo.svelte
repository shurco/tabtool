<script lang="ts">
	import { site } from '#lib/site';

	let {
		title,
		description = site.description,
		path = '/',
		image = '/img/og.png',
		type = 'website',
		jsonLd = []
	}: {
		title: string;
		description?: string;
		path?: string;
		image?: string;
		type?: string;
		jsonLd?: Record<string, unknown>[];
	} = $props();

	const canonical = $derived(`${site.url}${path}`);
	const ogImage = $derived(`${site.url}${image}`);
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta name="robots" content="index, follow, max-image-preview:large" />
	<meta property="og:site_name" content={site.name} />
	<meta property="og:type" content={type} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	<meta property="og:image" content={ogImage} />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:alt" content={title} />
	<meta property="og:locale" content="en_US" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={ogImage} />
	{#each jsonLd as data}
		{@html `<script type="application/ld+json">${JSON.stringify(data).replace(/</g, '\\u003c')}</script>`}
	{/each}
</svelte:head>
