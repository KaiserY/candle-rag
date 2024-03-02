import type { Config } from "tailwindcss";

export default {
	content: ["./index.html", "./src/**/*.{ts,tsx,css}"],
	theme: {
		extend: {},
	},
	plugins: [require("@tailwindcss/typography"), require("daisyui")],
} satisfies Config;
