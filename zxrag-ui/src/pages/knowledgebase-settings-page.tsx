import { Link, useLocation } from "react-router-dom";
import { InboxIcon } from "lucide-react";
import { TrashIcon } from "@radix-ui/react-icons";
import { useState, useEffect } from "react";

import { cn } from "@/lib/utils";
import { Separator } from "@/components/ui/separator";
import { buttonVariants } from "@/components/ui/button";
import { FileTable } from "@/components/knowledgebase-embeddings-page/file-table";
import { VectorTable } from "@/components/knowledgebase-embeddings-page/vector-table";

export function KnowledgebaseSettingsPage() {
	const [files, setFiles] = useState<File[]>([]);

	useEffect(() => {
		const listFiles = async () => {
			try {
				const response = await fetch("/v1/knowledgebases");

				if (!response.ok) {
					throw new Error(`HTTP error! status: ${response.status}`);
				}

				const responseJson = await response.json();

				console.log(responseJson);
			} catch (error) {
				console.error("Fetch error: ${error}");
			}
		};

		listFiles();
	}, []);

	return (
		<>
			<div className="container h-full py-6 overflow-hidden">
				<div className="flex h-full gap-6">
					<div className="group flex flex-col basis-2/12">
						<nav className="grid gap-1">
							<Link
								key=""
								to="#"
								className={cn(
									buttonVariants({ variant: "secondary", size: "default" }),
									true &&
										"dark:bg-muted dark:text-white dark:hover:bg-muted dark:hover:text-white",
									"justify-start",
								)}
							>
								<InboxIcon className="mr-2 h-4 w-4" />
								AA
								<span className={cn("ml-auto")}>
									<TrashIcon className="h-4 w-4 text-red-500" />
								</span>
							</Link>
							<Link
								key=""
								to="#"
								className={cn(
									buttonVariants({ variant: "default", size: "default" }),
									false &&
										"dark:bg-muted dark:text-white dark:hover:bg-muted dark:hover:text-white",
									"justify-start",
								)}
							>
								<InboxIcon className="mr-2 h-4 w-4" />
								CC
								<span
									className={cn(
										"ml-auto",
										false && "text-background dark:text-white",
									)}
								>
									DD
								</span>
							</Link>
						</nav>
					</div>
					<Separator orientation="vertical" />
					<div className="flex flex-col basis-8/12 space-y-4 max-h-full overflow-auto">
						<div>
							<h3 className="text-lg font-medium">Embedding</h3>
							<p className="text-sm text-muted-foreground">
								This is how others will see you on the site.
							</p>
						</div>
						<Separator orientation="horizontal" />
						<div>
							<h3 className="text-lg font-medium">File</h3>
							<FileTable />
						</div>
						<Separator orientation="horizontal" />
						<div>
							<h3 className="text-lg font-medium">Vector</h3>
							<VectorTable />
						</div>
					</div>
				</div>
			</div>
		</>
	);
}
