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
