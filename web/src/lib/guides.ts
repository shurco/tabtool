import { guide as csvTooLargeForExcelMac } from './guide-pages/csv-too-large-for-excel-mac';

export type GuideLink = { label: string; url: string };

export type GuideOption = {
	name: string;
	summary: string;
	good: string;
	cost: string;
	code?: string;
	link?: GuideLink;
};

export type Guide = {
	slug: string;
	title: string;
	metaTitle: string;
	metaDescription: string;
	checked: string;
	lede: string;
	image?: { src: string; srcset: string; alt: string; width: number; height: number };
	facts: { label: string; value: string }[];
	problem: string[];
	options: GuideOption[];
	benchmark?: { label: string; value: string; note: string }[];
	sources: GuideLink[];
};

export const guides: Guide[] = [csvTooLargeForExcelMac];

export const guideBySlug = (slug: string) => guides.find((guide) => guide.slug === slug);
