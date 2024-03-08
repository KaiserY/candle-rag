import { Link, useLocation } from "react-router-dom";

import { cn } from "@/lib/utils";

export function MainNav({
	className,
	...props
}: React.HTMLAttributes<HTMLElement>) {
	const location = useLocation();

	return (
		<nav
			className={cn("flex items-center space-x-4 lg:space-x-6 mx-6", className)}
			{...props}
		>
			<Link
				to="/chat"
				className={`text-lg font-medium transition-colors px-4 rounded-full hover:text-primary ${
					location.pathname === "/chat" || location.pathname === "/"
						? " bg-muted"
						: "text-muted-foreground"
				}`}
			>
				Chat
			</Link>
			<Link
				to="/knowledgebase"
				className={`text-lg font-medium transition-colors px-4 rounded-full hover:text-primary ${
					location.pathname === "/knowledgebase" ? " bg-muted" : "text-muted-foreground"
				}`}
			>
				Knowledgebase
			</Link>
			<Link
				to="/ocr"
				className={`text-lg font-medium transition-colors px-4 rounded-full hover:text-primary ${
					location.pathname === "/ocr" ? " bg-muted" : "text-muted-foreground"
				}`}
			>
				OCR
			</Link>
		</nav>
	);
}
