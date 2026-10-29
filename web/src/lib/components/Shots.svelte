<script lang="ts">
	import { shots } from '#lib/content';
</script>

<div class="shots">
	{#each shots as shot, i (shot.src)}
		<figure class="shot" class:shot--wide={i === shots.length - 1}>
			<div class="shot__frame">
				<img
					src={shot.src}
					srcset={shot.srcset}
					sizes={i === shots.length - 1 ? '(max-width: 760px) 100vw, 900px' : '(max-width: 760px) 100vw, 600px'}
					width={shot.width}
					height={shot.height}
					alt={shot.alt}
					loading="lazy"
					decoding="async"
				/>
			</div>
			<figcaption>
				<p class="shot__title">{shot.title}</p>
				<p class="shot__caption">{shot.caption}</p>
			</figcaption>
		</figure>
	{/each}
</div>

<style>
	.shots {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: clamp(18px, 2.4vw, 32px);
	}

	.shot {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.shot--wide {
		grid-column: 1 / -1;
		width: 100%;
		max-width: 760px;
		margin-inline: auto;
	}

	.shot__frame {
		padding: clamp(8px, 1.2vw, 14px);
		border: 1px solid var(--line);
		background: var(--paper-sunk);
		border-radius: var(--radius);
	}

	.shot__frame img {
		width: 100%;
		height: auto;
		border-radius: 2px;
	}

	.shot__title {
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--ink);
	}

	.shot__caption {
		margin-top: 6px;
		font-size: 14.5px;
		line-height: 1.5;
		color: var(--muted);
		max-width: 56ch;
	}

	@media (max-width: 760px) {
		.shots {
			grid-template-columns: 1fr;
		}
		.shot--wide {
			grid-column: auto;
		}
	}
</style>
