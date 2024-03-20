import { Link } from "react-router-dom";
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
	const [selectedknowledgeBase, setSelectedknowledgeBase] =
		useState<KnowledgeBase>({ id: 0, name: "", created_at: 0, updated_at: 0 });

	useEffect(() => {
		listKnowledgeBases();
	}, []);

	const listKnowledgeBases = async () => {
		try {
			const response = await fetch("/v1/knowledgebases");

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const responseJson = await response.json();

			const knowledgeBases: KnowledgeBase[] = responseJson.data;

			setKnowledgeBases(knowledgeBases);

			if (selectedknowledgeBase.id === 0 && knowledgeBases.length > 0) {
				setSelectedknowledgeBase(knowledgeBases[0]);
			}
		} catch (error) {
			console.error(`Fetch error: ${error}`);
		}
	};

	const create_knowledge_base = async (kb_name: string) => {
		try {
			const response = await fetch("/v1/knowledgebases", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({ name: kb_name }),
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const responseJson = await response.json();

			console.log(responseJson);

			listKnowledgeBases();

			setSelectedknowledgeBase({
				id: responseJson.id,
				name: responseJson.name,
				created_at: 0,
				updated_at: 0,
			});
		} catch (error) {
			console.error(`Fetch error: ${error}`);
		}
	};

	const delete_knowledge_base = async (kb_id: number) => {
		try {
			const response = await fetch(`/v1/knowledgebases/${kb_id}`, {
				method: "DELETE",
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const responseJson = await response.json();

			console.log(responseJson);

			listKnowledgeBases();
		} catch (error) {
			console.error(`Fetch error: ${error}`);
		}
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
										key={kb.id}
										to="#"
										className={cn(
											buttonVariants({
												variant:
													selectedknowledgeBase.id === kb.id
														? "secondary"
														: "ghost",
												size: "default",
											}),
										)}
										onClick={() => {
											setSelectedknowledgeBase(kb);
										}}
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
													<DialogTitle>Delete KB "{kb.name}"</DialogTitle>
													<DialogDescription>Are you sure?</DialogDescription>
												</DialogHeader>
												<DialogFooter>
													<Button
														variant="destructive"
														type="button"
														onClick={(e) => {
															e.preventDefault();
															delete_knowledge_base(kb.id);
															setDeleteKbOpen(false);
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
												e.preventDefault();
												create_knowledge_base(createKbName);
												setCreateKbOpen(false);
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
							<h3 className="text-lg font-medium">
								Knowledge Base {selectedknowledgeBase.name}
							</h3>
							<p className="text-sm text-muted-foreground">
								This is how others will see you on the site.
							</p>
						</div>
						<Separator orientation="horizontal" />
						<div>
							<h3 className="text-lg font-medium">File</h3>
							<FileTable selectedknowledgeBase={selectedknowledgeBase} />
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
