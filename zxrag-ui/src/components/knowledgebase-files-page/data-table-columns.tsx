import { ColumnDef } from "@tanstack/react-table";

import { DataTableColumnHeader } from "@/components/knowledgebase-files-page/data-table-column-header";
import { DataTableRowActions } from "@/components/knowledgebase-files-page/data-table-row-actions";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { File } from "@/schema";

export const columns: ColumnDef<File>[] = [
	{
		id: "select",
		header: ({ table }) => (
			<Checkbox
				checked={table.getIsAllPageRowsSelected()}
				onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
				aria-label="Select all"
				className="translate-y-[2px]"
			/>
		),
		cell: ({ row }) => (
			<Checkbox
				checked={row.getIsSelected()}
				onCheckedChange={(value) => row.toggleSelected(!!value)}
				aria-label="Select row"
				className="translate-y-[2px]"
			/>
		),
		enableSorting: false,
		enableHiding: false,
	},
	{
		id: "id",
		accessorKey: "id",
		header: ({ column }) => (
			<DataTableColumnHeader column={column} title="Id" />
		),
		cell: ({ row }) => <div className="w-[100px]">{row.getValue("id")}</div>,
		enableSorting: false,
		enableHiding: false,
	},
	{
		id: "filename",
		accessorKey: "filename",
		header: ({ column }) => (
			<DataTableColumnHeader column={column} title="Filename" />
		),
		cell: ({ row }) => {
			return (
				<div className="flex space-x-2">
					<Badge variant="outline">TXT</Badge>
					<span className="max-w-[500px] truncate font-medium">
						{row.getValue("filename")}
					</span>
				</div>
			);
		},
	},
	{
		id: "size",
		accessorKey: "bytes",
		header: ({ column }) => (
			<DataTableColumnHeader column={column} title="Size" />
		),
		cell: ({ row }) => {
			return (
				<div className="flex w-[100px] items-center">
					<span>{row.getValue("size")}</span>
				</div>
			);
		},
		filterFn: (row, id, value) => {
			return value.includes(row.getValue(id));
		},
	},
	{
		id: "created",
		accessorKey: "created_at",
		header: ({ column }) => (
			<DataTableColumnHeader column={column} title="Created" />
		),
		cell: ({ row }) => {
			return (
				<div className="flex items-center">
					<span>{row.getValue("created")}</span>
				</div>
			);
		},
		filterFn: (row, id, value) => {
			return value.includes(row.getValue(id));
		},
	},
	{
		id: "actions",
		cell: ({ row }) => <DataTableRowActions row={row} />,
	},
];
