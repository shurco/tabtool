<script lang="ts">
	import '../app.css';
	import Header from '#lib/components/Header.svelte';
	import Footer from '#lib/components/Footer.svelte';
	import DownloadBar from '#lib/components/DownloadBar.svelte';

	let { children } = $props();
</script>

<a class="skip" href="#main">Skip to content</a>
<Header />
<main id="main" tabindex="-1">
	{@render children()}
</main>
<Footer />
<DownloadBar />
