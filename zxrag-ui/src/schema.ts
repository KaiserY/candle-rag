import { z } from "zod";

export const fileSchema = z.object({
	id: z.string(),
	bytes: z.number(),
	created_at: z.number(),
	filename: z.string(),
	object: z.string(),
	purpose: z.string(),
});

export type File = z.infer<typeof fileSchema>;

export const knowledgeBaseSchema = z.object({
	id: z.number(),
	name: z.string(),
	created_at: z.number(),
	updated_at: z.number(),
});

export type KnowledgeBase = z.infer<typeof knowledgeBaseSchema>;
