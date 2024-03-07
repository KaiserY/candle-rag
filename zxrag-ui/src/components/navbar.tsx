import { LanguageToggle } from "@/components/language-toggle";
import { Logo } from "@/components/logo";
import { MainNav } from "@/components/main-nav";
import { ModeToggle } from "@/components/mode-toggle";
import { cn } from "@/lib/utils";

interface NavbarProps extends React.HTMLAttributes<HTMLDivElement> {}

export function Navbar({ className }: NavbarProps) {
	return (
		<div
			className={cn(
				"flex flex-row justify-between h-16 items-center px-4",
				className,
			)}
		>
			<Logo className="flex order-1" />
			<div className="flex grow order-2">
				<MainNav className="mx-6" />
			</div>
			<div className="flex grow order-3" />
			<div className="flex items-center order-11 gap-2">
				<LanguageToggle />
				<ModeToggle />
			</div>
		</div>
	);
}
