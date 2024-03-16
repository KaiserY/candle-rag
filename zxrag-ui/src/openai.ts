import OpenAI from "openai";

export const openai = new OpenAI({
	baseURL: `${window.location.protocol}//${window.location.host}/v1`,
	apiKey: "NULL",
	dangerouslyAllowBrowser: true,
});
