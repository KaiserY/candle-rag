import { useState, useEffect } from "react";

import { DataTable } from "@/components/knowledgebase-files-page/data-table";
import { columns } from "@/components/knowledgebase-files-page/data-table-columns";
import { File } from "@/schema";
import { openai } from "@/openai";

const aa = [
	{
		id: "TASK-8782",
		filename: "aa.txt",
		bytes: 111,
		created_at: 222,
		object: "file",
		purpose: "embedding",
	},
];

export function KnowledgebaseFilesPage() {
	const [files, setFiles] = useState<File[]>([]);

	useEffect(() => {
		const listFiles = async () => {
			const response = await openai.files.list();

			console.log(response.data);

			setFiles(response.data);
		};

		listFiles();
	}, []);

	return (
		<>
			<div className="container h-full py-6">
				<DataTable data={files} columns={columns} />
			</div>
		</>
	);
}
