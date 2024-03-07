import i18next from "i18next";
import { Languages } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export function LanguageToggle() {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant="ghost" size="icon">
					<Languages className="h-[1.2rem] w-[1.2rem] rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
					<span className="sr-only">Toggle language</span>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end">
				<DropdownMenuItem
					onClick={() => {
						i18next.changeLanguage("en");
					}}
				>
					English
				</DropdownMenuItem>
				<DropdownMenuItem
					onClick={() => {
						i18next.changeLanguage("cn");
					}}
				>
					简体中文
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
