import { Boxes } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

export function Logo({
	className,
	...props
}: React.HTMLAttributes<HTMLDivElement>) {
	const { t } = useTranslation();

	return (
		<div className={cn("flex", className)} {...props}>
			<Boxes className="mr-4" />
			<span className="text-lg font-medium">{t("title")}</span>
		</div>
	);
}
