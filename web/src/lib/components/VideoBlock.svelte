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
			height="860"
			playsinline
			aria-label="Screen recording of TabTool opening and scrolling a 2.1-million-row CSV"
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
			>Unedited capture · macOS build · 2,100,000 rows × 7 columns · 109 MB · 2026-10-29</span
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
