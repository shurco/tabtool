# TabTool promotional site

The marketing site for [TabTool](../README.md), built with SvelteKit and prerendered to static files for Cloudflare Pages.

## What is here

- `/` — overview: hero, measured numbers, features, demo video, real screenshots, open-source ask, platform roadmap, comparison index, FAQ.
- `/download/` — App Store links per platform, requirements, and build-from-source instructions.
- `/vs/` and `/vs/<slug>/` — five comparison pages: TableTool (TablePlus), the original open-source TableTool, Modern CSV, CSViewer, Tablecruncher.
- `/faq/` — the full FAQ, also emitted as `FAQPage` structured data.
- `/press/` — press kit: boilerplate, fast facts, asset links, and contact.
- `/llms.txt` — factual product summary for AI assistants and agents.
- `/feedback/` — how to send feedback: GitHub issues link and what to include (no email or form, the site collects nothing).
- `/privacy/`, `/terms/` — Privacy Policy and User Agreement.
- `/sitemap.xml`, `/robots.txt` — generated at build time.
- `/404.html` — static not-found page.

Screenshots and the demo video are real captures of the development build, taken 29 October 2026. No composited UI.

## Local development

```sh
cd web
npm install
npm run dev
```

Checks and a production build:

```sh
npm run check
npm run build
npm run preview
```

The build writes static files to `web/build`.

## Deploy to Cloudflare Pages

Connect the GitHub repository in the Cloudflare dashboard and use:

| Setting | Value |
| --- | --- |
| Framework preset | SvelteKit |
| Root directory | `web` |
| Build command | `npm run build` |
| Build output directory | `build` |
| Environment variable | `NODE_VERSION=22` |

### Workers Builds

If the Cloudflare project uses Workers Builds (it runs a deploy command after the build), keep the build command above and the deploy command `npm run deploy` (runs `wrangler deploy`) works as is. `wrangler.jsonc` serves `build/` as static assets; no Worker code is needed.

For Pages, the site is fully prerendered, so Pages serves `build/` as static assets. `static/_headers` sets security and cache headers, and `static/_redirects` keeps old legal and comparison URLs working.

## Edit before launch

Three placeholders must be replaced:

1. **GitHub URL** — `src/lib/site.ts`, the `github` field. It currently points to `https://github.com/shurco/tabtool`.
2. **Site origin** — `src/lib/site.ts` (`url`) and `vite.config.ts` (`paths.origin`). It currently uses `https://tabtool.shurco.app`; use the real domain before submitting the sitemap.
3. **App Store provider token** — `src/lib/site.ts`, `appStore.providerToken`. Fill the nine-digit token from App Store Connect to attribute installs to site placements. Without it, the links still work but Apple cannot attribute the install.

Everything else (comparison facts, FAQ, legal text, screenshots) lives in `src/lib/`:

- `site.ts` — name, description, links (GitHub, App Store `appStore.mac`/`appStore.ios`), navigation.
- `content.ts` — stats, features, screenshots, platforms, FAQ.
- `comparisons.ts` — one object per comparison page.

## App Store badges

- `static/img/badges/mac-app-store.svg` and `app-store.svg` are Apple's official black US-English badges from the [App Store marketing guidelines](https://developer.apple.com/app-store/marketing/guidelines/). Do not edit the artwork.
- `src/lib/components/StoreBadges.svelte` renders them as links to `site.appStore`; pass `platforms={['mac']}` or `['ios']` to show one. It is used in the home hero, the CTA, and the download page.

## Analytics

- Google Analytics 4 (`G-G8JX3V6FNB`) is loaded via `gtag.js` in `src/app.html`. The privacy page discloses it; keep both in sync.
- Every App Store link goes through `appStoreURL()` in `src/lib/track.ts` and fires GA4 `app_store_click` with `platform` (`mac`/`ios`) and `placement` (`hero`, `cta`, `download`, `header-bar`, `compare`, `about`). The link also carries Apple's `pt`, `ct`, and `mt` campaign parameters.
- The sticky mobile download bar is dismissible; the choice is stored in `sessionStorage` for the session.

## SEO

- Every page sets a unique title, description, canonical URL, Open Graph, and Twitter card.
- `SoftwareApplication`, `WebSite`, `FAQPage`, `BreadcrumbList`, and `CollectionPage` JSON-LD are generated from the same content as the pages.
- `sitemap.xml` and `robots.txt` are prerendered endpoints; both use the `url` constant.
- `llms.txt` is a prerendered plain-text summary for AI assistants and agents. Keep its facts identical to the README.
- Internal links use trailing slashes consistently, matching `trailingSlash = 'always'`.
- Fonts and media are self-hosted; images are WebP with `srcset`, dimensions, and lazy loading below the fold.

## Notes

- The legal pages describe the current static site and the app's behavior. They are project text, not legal advice; have a lawyer review them before relying on them commercially.
- Comparison pages cite vendor pages from October 2026 and include a "where the other app is stronger" section on purpose. Update the facts when a vendor changes something material.
