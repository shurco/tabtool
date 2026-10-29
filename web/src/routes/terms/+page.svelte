<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import { site } from '#lib/site';

	const title = 'User Agreement — TabTool';
	const description =
		'The terms for using this website and the TabTool application: MIT license, no warranty, acceptable use, and third-party links.';
</script>

<Seo {title} {description} path="/terms/" />

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Legal</p>
		<h1 class="legal__title">User Agreement</h1>
		<p class="legal__updated">Effective 29 October 2026 · Last updated 29 October 2026</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="legal">
			<p>
				This agreement covers your use of this website and the TabTool application. By using either,
				you accept it. If you do not, do not use them.
			</p>

			<h2>1. The app is MIT-licensed</h2>
			<p>
				The TabTool source code is released under the
				<a href={`${site.github}/blob/HEAD/LICENSE`} rel="noopener">MIT License</a>. That license is
				the operative grant for the code: you may use, copy, modify, merge, publish, distribute,
				sublicense, and sell copies, provided the copyright and permission notice is included. Nothing
				in this agreement narrows the rights the MIT License gives you.
			</p>
			<p>
				A build distributed through an app store may also be subject to that store’s standard terms.
				Where those terms conflict with the MIT License for the code itself, the MIT License governs
				the code.
			</p>

			<h2>2. No warranty</h2>
			<p>
				The software and this website are provided <strong>“as is”, without warranty of any kind</strong>,
				express or implied, including warranties of merchantability, fitness for a particular purpose,
				and non-infringement. You are responsible for keeping backups. CSV editing can change files;
				verify important data before and after saving.
			</p>

			<h2>3. Limitation of liability</h2>
			<p>
				To the maximum extent permitted by law, the authors and contributors are not liable for any
				claim, damages, or other liability, whether in contract, tort, or otherwise, arising from,
				out of, or in connection with the software, the website, or their use. This includes lost
				data, lost profits, and business interruption.
			</p>
			<p>
				Nothing in this agreement excludes liability that cannot be excluded under applicable law,
				including mandatory consumer rights.
			</p>

			<h2>4. Acceptable use of the website</h2>
			<ul>
				<li>Do not attempt to disrupt the site or the host’s infrastructure.</li>
				<li>Do not misrepresent the project or its licenses.</li>
				<li>Do not use the project’s name or marks to imply endorsement without permission.</li>
			</ul>

			<h2>5. Content and marks</h2>
			<p>
				The text, layout, and images on this site are © 2026 the TabTool contributors, except where a
				different source is credited. The code is MIT-licensed; the site’s editorial content is not
				automatically covered by that license. “TabTool” is used as the project name.
			</p>
			<p>
				Product names and logos of other applications mentioned in comparisons belong to their
				owners. They are used for identification and comparison only, and do not imply affiliation or
				endorsement.
			</p>

			<h2>6. Third-party links</h2>
			<p>
				The site links to GitHub, vendor websites, and app stores. Those services are not controlled by
				the project, and their terms and privacy policies apply when you visit them.
			</p>

			<h2>7. Changes</h2>
			<p>
				This agreement may change as the project evolves. The effective date at the top will change,
				and the edit will be visible in the repository history. Continued use after a change means you
				accept the updated agreement.
			</p>

			<h2>8. Severability and governing law</h2>
			<p>
				If a provision is found unenforceable, the remaining provisions stay in effect. These terms are
				governed by the laws applicable at the maintainer’s principal place of business, without regard
				to conflict-of-law rules, and without limiting mandatory protections available where you live.
			</p>

			<h2>9. Contact</h2>
			<p>
				Questions about this agreement: open an issue at
				<a href={`${site.github}/issues`} rel="noopener">{site.github}/issues</a>.
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
