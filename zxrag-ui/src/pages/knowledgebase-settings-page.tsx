import { Link, useLocation } from "react-router-dom";
import { InboxIcon } from "lucide-react";
import { PlusCircledIcon, TrashIcon } from "@radix-ui/react-icons";
import { useState, useEffect } from "react";

import { cn } from "@/lib/utils";
import { KnowledgeBase } from "@/schema";
import { Separator } from "@/components/ui/separator";
import { buttonVariants, Button } from "@/components/ui/button";
import { FileTable } from "@/components/knowledgebase-settings-page/file-table";
import { VectorTable } from "@/components/knowledgebase-settings-page/vector-table";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function KnowledgebaseSettingsPage() {
	const [knowledgeBases, setKnowledgeBases] = useState<KnowledgeBase[]>([]);
	const [createKbName, setCreateKbName] = useState("");
	const [createKbOpen, setCreateKbOpen] = useState(false);
	const [deleteKbOpen, setDeleteKbOpen] = useState(false);

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

		setKnowledgeBases([
			{
				id: 1,
				name: "aa",
				created_at: 1,
				updated_at: 1,
			},
		]);

		listFiles();
	}, []);

	const create_knowledge_base = async (kb_name: number) => {
		console.log(kb_name);
	};

	const delete_knowledge_base = async (kb_id: number) => {
		console.log(kb_id);
	};

	return (
		<>
			<div className="container h-full py-6 overflow-hidden">
				<div className="flex h-full gap-4">
					<div className="group flex flex-col basis-2/12 px-2">
						<nav className="grid gap-1">
							{knowledgeBases.map((kb) => {
								return (
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
										{kb.name}

										<Dialog open={deleteKbOpen} onOpenChange={setDeleteKbOpen}>
											<DialogTrigger asChild>
												<Label className="ml-auto">
													<TrashIcon className="h-4 w-4 text-red-500" />
												</Label>
											</DialogTrigger>
											<DialogContent className="sm:max-w-[425px]">
												<DialogHeader>
													<DialogTitle>Delete KB</DialogTitle>
													<DialogDescription>Are you sure?</DialogDescription>
												</DialogHeader>
												<DialogFooter>
													<Button
														variant="destructive"
														type="button"
														onClick={(e) => {
															setDeleteKbOpen(false);
															e.preventDefault();
															console.log(e.target);
														}}
													>
														Delete
													</Button>
												</DialogFooter>
											</DialogContent>
										</Dialog>
									</Link>
								);
							})}

							<Dialog open={createKbOpen} onOpenChange={setCreateKbOpen}>
								<DialogTrigger asChild>
									<Button variant="ghost" className={cn("justify-start")}>
										<PlusCircledIcon className="mr-2 h-4 w-4" />
										New Knowledge Base
									</Button>
								</DialogTrigger>
								<DialogContent className="sm:max-w-[425px]">
									<DialogHeader>
										<DialogTitle>Create KB</DialogTitle>
										<DialogDescription>Create KB</DialogDescription>
									</DialogHeader>
									<div className="grid grid-cols-4 items-center gap-4">
										<Label htmlFor="name" className="text-right">
											Name
										</Label>
										<Input
											placeholder="KB name"
											className="col-span-3"
											value={createKbName}
											onChange={(e) => {
												setCreateKbName(e.target.value);
											}}
										/>
									</div>
									<DialogFooter>
										<Button
											variant="default"
											type="button"
											onClick={(e) => {
												setCreateKbOpen(false);
												e.preventDefault();
												console.log(e.target);
											}}
										>
											Create KB
										</Button>
									</DialogFooter>
								</DialogContent>
							</Dialog>
						</nav>
					</div>
					<Separator orientation="vertical" />
					<div className="flex flex-col basis-8/12 space-y-4 px-2 max-h-full overflow-auto">
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
