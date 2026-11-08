<script lang="ts">
	let { src = '/media/tabtool-demo.mp4', poster = '/media/tabtool-demo-poster.jpg' } = $props();
</script>

<figure class="demo">
	<div class="demo__frame">
		<video
			controls
			preload="none"
			poster={poster}
			width="1400"
			height="644"
			playsinline
			aria-label="Screen recording of TabTool on iPhone opening and scrolling an emoji CSV"
		>
			<source {src} type="video/mp4" />
			<p>
				Your browser does not support video. <a href={src}>Download the recording</a>.
			</p>
		</video>
	</div>
	<figcaption class="demo__caption">
		<span class="demo__dot" aria-hidden="true"></span>
		<span class="mono"
			>Unedited capture · iPhone build · large-file mode · 1,000,000 rows × 12 columns · 2026-11-08</span
		>
	</figcaption>
</figure>

<style>
	.demo {
		margin: 0;
	}

	.demo__frame {
		padding: clamp(10px, 1.6vw, 20px);
		background: var(--paper-sunk);
		border: 1px solid var(--line);
		border-radius: var(--radius);
	}

	.demo video {
		width: 100%;
		border-radius: 2px;
		background: var(--paper-raised);
		box-shadow: 0 18px 40px -28px rgb(20 21 15 / 0.5);
	}

	.demo__caption {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 14px;
		color: var(--muted);
	}

	.demo__dot {
		display: inline-block;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
		flex: 0 0 auto;
	}
</style>
