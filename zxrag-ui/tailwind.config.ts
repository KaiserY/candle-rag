import type { Config } from "tailwindcss";

export default {
	content: ["./index.html", "./src/**/*.{css}", "./src/**/*.{ts,tsx}"],
	theme: {
		extend: {},
	},
	plugins: [require("daisyui")],
} satisfies Config;
