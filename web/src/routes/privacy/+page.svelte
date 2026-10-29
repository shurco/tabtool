<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import { site } from '#lib/site';

	const title = 'Privacy Policy — TabTool';
	const description =
		'TabTool collects nothing: no cookies, no analytics, no accounts, no network entitlement in the app. How the website and the app handle data.';
</script>

<Seo {title} {description} path="/privacy/" />

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Legal</p>
		<h1 class="legal__title">Privacy Policy</h1>
		<p class="legal__updated">Effective 29 October 2026 · Last updated 29 October 2026</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="legal">
			<p>
				This policy covers two things: this website and the TabTool application. They are handled
				differently, because they are built differently. The short version: the website sets no
				cookies and runs no analytics, and the app has no network access at all.
			</p>

			<h2>1. Who is responsible</h2>
			<p>
				TabTool is an open-source project maintained by its contributors. The source code and the
				issue tracker are on <a href={site.github} rel="noopener">GitHub</a>. For any privacy
				question, open an issue there; it is the only contact channel the project publishes.
			</p>

			<h2>2. The website</h2>
			<h3>What we do not do</h3>
			<ul>
				<li>No analytics, session recording, heat maps, or A/B testing scripts.</li>
				<li>No advertising pixels or social embeds.</li>
				<li>No third-party fonts or video host. Fonts and the demo video are served from this domain.</li>
				<li>No accounts, newsletters, or forms that ask for an email address.</li>
				<li>
					No cookies of our own. The site is static, so there is nothing to keep in a session; the
					host’s bot protection may set a security cookie when it challenges automated traffic.
				</li>
			</ul>

			<h3>What the host sees</h3>
			<p>
				The site is served by Cloudflare Pages. Like any web host, Cloudflare processes technical
				request data to deliver the site and protect it from abuse: IP address, user agent, requested
				URL, referrer, and timestamps. That processing happens under
				<a href="https://www.cloudflare.com/privacypolicy/" rel="noopener">Cloudflare’s privacy policy</a>.
				The project may see aggregate traffic counts in the host dashboard, but it does not receive
				individual request records, combine them, or use them to identify visitors.
			</p>

			<h3>External links</h3>
			<p>
				Links to GitHub, vendor websites, and the App Store leave this site. Those services apply
				their own policies, which are outside this document.
			</p>

			<h3>Legal basis and retention</h3>
			<p>
				Where the GDPR applies, the legal basis for serving the site is legitimate interest in
				operating and securing a public website. Retention of request logs is determined by
				Cloudflare, not by the project. Because the project collects no personal data of its own,
				there is no project-side retention period and nothing to delete on request.
			</p>

			<h2>3. The TabTool application</h2>
			<h3>Your files</h3>
			<p>
				TabTool opens documents you choose and writes them back to the same place. The app is
				sandboxed and has no network entitlement. No file, path, column name, or cell value is
				transmitted anywhere.
			</p>

			<h3>Settings</h3>
			<p>
				Preferences such as language, row height, default format, and the large-file threshold are
				stored locally in the app’s own preferences container. They are not synced and are removed
				when you delete the app.
			</p>

			<h3>Diagnostics</h3>
			<p>
				The app contains no analytics SDK and no third-party crash reporter. If you have opted in to
				share diagnostics with Apple at the system level, macOS and iOS may collect crash reports
				under Apple’s terms; that is controlled by your device settings, not by TabTool.
			</p>

			<h3>Distribution and purchases</h3>
			<p>
				If you install TabTool from the App Store, Apple processes the download and any purchase
				under its own privacy policy. The project does not receive your name, email address, or
				payment details. Direct downloads from the repository involve no account and no telemetry.
			</p>

			<h2>4. Children</h2>
			<p>
				The site and the app are not directed at children, and neither collects personal data from
				anyone, including children.
			</p>

			<h2>5. Your rights</h2>
			<p>
				Depending on where you live, you may have rights to access, correct, delete, or port personal
				data, and to object to processing. The project holds no personal data about visitors or app
				users, so there is nothing to access or erase. If you believe a specific request log held by
				the host concerns you, contact Cloudflare as the processor; the project cannot retrieve it.
			</p>

			<h2>6. Changes</h2>
			<p>
				If this policy changes, the effective date at the top changes with it, and the edit is visible
				in the repository history. Material changes will be noted in the repository.
			</p>

			<h2>7. Contact</h2>
			<p>
				Open an issue at <a href={`${site.github}/issues`} rel="noopener">{site.github}/issues</a>.
			</p>
		</div>
	</div>
</section>

<style>
	.legal__title {
		margin-top: 16px;
		font-size: clamp(2.6rem, 6vw, 4.4rem);
	}

	.legal__updated {
		margin-top: 14px;
	}
</style>
