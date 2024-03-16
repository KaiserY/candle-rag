import { Cross2Icon, UploadIcon } from "@radix-ui/react-icons";
import { Table } from "@tanstack/react-table";
import { useState, useRef } from "react";

import { DataTableViewOptions } from "@/components/knowledgebase-files-page/data-table-view-options";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { openai } from "@/openai";

interface DataTableToolbarProps<TData> {
	table: Table<TData>;
}

export function DataTableToolbar<TData>({
	table,
}: DataTableToolbarProps<TData>) {
	const [isLoading, setIsLoading] = useState<boolean>(false);

	const inputRef = useRef<HTMLInputElement>(null);

	const isFiltered = table.getState().columnFilters.length > 0;

	const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
		console.log(e.target.files);

		if (e.target.files != null && e.target.files.length > 0) {
			await openai.files.create({
				file: e.target.files[0],
				purpose: "fine-tune",
			});
		}
	};

	return (
		<div className="flex items-center justify-between">
			<div className="flex flex-1 items-center space-x-2">
				<Button
					variant="secondary"
					className="h-8 px-2 lg:px-3"
					onClick={(e) => {
						e.preventDefault();
						inputRef.current?.click();
					}}
				>
					<UploadIcon className="mr-2 h-4 w-4" />
					Upload
				</Button>
				<Input
					id="file"
					type="file"
					className="hidden"
					ref={inputRef}
					onChange={handleChange}
				/>
				<Input
					placeholder="Filter tasks..."
					value={
						(table.getColumn("filename")?.getFilterValue() as string) ?? ""
					}
					onChange={(event) =>
						table.getColumn("filename")?.setFilterValue(event.target.value)
					}
					className="h-8 w-[150px] lg:w-[250px]"
				/>
				{isFiltered && (
					<Button
						variant="ghost"
						onClick={() => table.resetColumnFilters()}
						className="h-8 px-2 lg:px-3"
					>
						Reset
						<Cross2Icon className="ml-2 h-4 w-4" />
					</Button>
				)}
			</div>
			<DataTableViewOptions table={table} />
		</div>
	);
}
