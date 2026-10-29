<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import { site } from '#lib/site';

	const title = 'Feedback — TabTool';
	const description =
		'Report a bug, request a feature, or share an idea about TabTool. Feedback goes through GitHub issues; the app and this site collect nothing.';
	const issues = `${site.github}/issues/new`;
</script>

<Seo {title} {description} path="/feedback/" />

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Contact</p>
		<h1 class="legal__title">Feedback</h1>
		<p class="legal__updated">Bugs, ideas, and questions are all welcome.</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="legal">
			<p>
				TabTool has no accounts, no in-app forms, and no telemetry, so nothing reaches us unless you
				send it. The one channel is GitHub issues: public, searchable, and tracked.
			</p>

			<p>
				<a class="btn btn--primary" href={issues} rel="noopener">
					Open an issue <span class="btn__arrow" aria-hidden="true">↗</span>
				</a>
			</p>

			<h2>What to include</h2>
			<ul>
				<li>macOS or iOS version and the TabTool version;</li>
				<li>what you were doing and what you expected instead;</li>
				<li>file size, delimiter, and encoding if a CSV is involved.</li>
			</ul>

			<h2>Please do not attach</h2>
			<p>
				Real data. Cut the file down to a few rows with fake values that still reproduce the
				problem, since issues are public.
			</p>

			<h2>We are glad to hear about</h2>
			<ul>
				<li>bugs and crashes;</li>
				<li>files that open slowly or render wrongly;</li>
				<li>missing format or encoding support;</li>
				<li>accessibility and clarity of controls.</li>
			</ul>

			<p>
				Quick answers may already be in the <a href="/faq/">FAQ</a>. How data is handled:
				<a href="/privacy/">Privacy Policy</a>.
			</p>
		</div>
	</div>
</section>
