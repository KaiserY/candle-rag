"use client";

import * as React from "react";

import { KnowledgeBase } from "@/schema";

import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";

interface KnowledgeBaseSelectorProps {
	knowledgeBases: KnowledgeBase[];
	selectedknowledgeBase: KnowledgeBase;
	setSelectedknowledgeBase: React.Dispatch<React.SetStateAction<KnowledgeBase>>;
}

export function KnowledgeBaseSelector({
	knowledgeBases,
	selectedknowledgeBase,
	setSelectedknowledgeBase,
}: KnowledgeBaseSelectorProps) {
	return (
		<div className="grid gap-2 pt-2">
			<div className="grid gap-4">
				<div className="flex items-center justify-between">
					<Label htmlFor="maxlength">KnowledgeBase</Label>
				</div>
				<Select
					onValueChange={(e) => {
						setSelectedknowledgeBase(
							knowledgeBases.find((kb) => kb.name === e) || {
								id: 0,
								name: "",
								created_at: 0,
								updated_at: 0,
							},
						);
					}}
				>
					<SelectTrigger className="w-full">
						<SelectValue placeholder={selectedknowledgeBase.name} />
					</SelectTrigger>
					<SelectContent>
						<SelectGroup>
							<SelectLabel>KnowledgeBase</SelectLabel>
							{knowledgeBases.map((kb) => {
								return (
									<SelectItem key={kb.id} value={kb.name}>
										{kb.name}
									</SelectItem>
								);
							})}
						</SelectGroup>
					</SelectContent>
				</Select>
			</div>
		</div>
	);
}
