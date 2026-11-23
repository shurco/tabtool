import { guide as shopifyProductCsvWithoutExcel } from './guide-pages/shopify-product-csv-without-excel';
import { guide as csvEncodingGarbledExcelMac } from './guide-pages/csv-encoding-garbled-excel-mac';
import { guide as semicolonCsvMac } from './guide-pages/semicolon-csv-mac';
import { guide as csvTooBigForNumbers } from './guide-pages/csv-too-big-for-numbers';
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
	video?: { src: string; poster: string; width: number; height: number; label: string; caption: string };
	facts: { label: string; value: string }[];
	problem: string[];
	options: GuideOption[];
	benchmark?: { label: string; value: string; note: string }[];
	sources: GuideLink[];
};

export const guides: Guide[] = [csvTooLargeForExcelMac, csvTooBigForNumbers, semicolonCsvMac, csvEncodingGarbledExcelMac, shopifyProductCsvWithoutExcel];

export const guideBySlug = (slug: string) => guides.find((guide) => guide.slug === slug);
