import cn from "@/locales/cn.json";
import en from "@/locales/en.json";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";

i18next.use(initReactI18next).init({
	lng: "en",
	debug: true,
	resources: {
		en: {
			...en,
		},
		cn: {
			...cn,
		},
	},
});
