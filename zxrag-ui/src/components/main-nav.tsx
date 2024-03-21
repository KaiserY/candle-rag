import { Link, useLocation } from "react-router-dom";
import * as React from "react";

import { cn } from "@/lib/utils";
import {
	NavigationMenu,
	NavigationMenuContent,
	NavigationMenuItem,
	NavigationMenuLink,
	NavigationMenuList,
	NavigationMenuTrigger,
	navigationMenuTriggerStyle,
} from "@/components/ui/navigation-menu";
import {
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbList,
	BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";

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
			<NavigationMenu>
				<NavigationMenuList>
					<NavigationMenuItem>
						<Link
							to="/chat"
							className={cn(
								navigationMenuTriggerStyle(),
								location.pathname === "/chat" || location.pathname === "/"
									? "bg-accent text-accent-foreground"
									: "",
							)}
						>
							Chat
						</Link>
					</NavigationMenuItem>
					<NavigationMenuItem>
						<NavigationMenuTrigger
							className={cn(
								navigationMenuTriggerStyle(),
								location.pathname.startsWith("/knowledgebase/")
									? "bg-accent text-accent-foreground"
									: "",
							)}
						>
							<Breadcrumb>
								<BreadcrumbList>
									<BreadcrumbItem>Knowledge Base</BreadcrumbItem>
									{location.pathname === "/knowledgebase/chat" && (
										<>
											<BreadcrumbSeparator />
											<BreadcrumbItem>Chat</BreadcrumbItem>
										</>
									)}
									{location.pathname === "/knowledgebase/settings" && (
										<>
											<BreadcrumbSeparator />
											<BreadcrumbItem>Settings</BreadcrumbItem>
										</>
									)}
									{location.pathname === "/knowledgebase/files" && (
										<>
											<BreadcrumbSeparator />
											<BreadcrumbItem>Files</BreadcrumbItem>
										</>
									)}
								</BreadcrumbList>
							</Breadcrumb>
						</NavigationMenuTrigger>
						<NavigationMenuContent>
							<ul className="grid w-[400px] gap-3 p-4 md:w-[500px] md:grid-cols-2 lg:w-[600px] ">
								<li>
									<NavigationMenuLink asChild>
										<Link
											to="/knowledgebase/chat"
											className={cn(
												"block select-none space-y-1 rounded-md p-3 leading-none no-underline outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground",
												className,
											)}
										>
											<div className="text-sm font-medium leading-none">
												Chat
											</div>
											<p className="line-clamp-2 text-sm leading-snug text-muted-foreground">
												Knowledge Base Chat
											</p>
										</Link>
									</NavigationMenuLink>
								</li>
								<li>
									<NavigationMenuLink asChild>
										<Link
											to="/knowledgebase/settings"
											className={cn(
												"block select-none space-y-1 rounded-md p-3 leading-none no-underline outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground",
												className,
											)}
										>
											<div className="text-sm font-medium leading-none">
												Settings
											</div>
											<p className="line-clamp-2 text-sm leading-snug text-muted-foreground">
												Knowledge Base Settings
											</p>
										</Link>
									</NavigationMenuLink>
								</li>
								<li>
									<NavigationMenuLink asChild>
										<Link
											to="/knowledgebase/files"
											className={cn(
												"block select-none space-y-1 rounded-md p-3 leading-none no-underline outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground",
												className,
											)}
										>
											<div className="text-sm font-medium leading-none">
												Files
											</div>
											<p className="line-clamp-2 text-sm leading-snug text-muted-foreground">
												Knowledge Base Files
											</p>
										</Link>
									</NavigationMenuLink>
								</li>
							</ul>
						</NavigationMenuContent>
					</NavigationMenuItem>
					<NavigationMenuItem>
						<Link
							to="/ocr"
							className={cn(
								navigationMenuTriggerStyle(),
								location.pathname === "/ocr"
									? "bg-accent text-accent-foreground"
									: "",
							)}
						>
							OCR
						</Link>
					</NavigationMenuItem>
				</NavigationMenuList>
			</NavigationMenu>
		</nav>
	);
}
