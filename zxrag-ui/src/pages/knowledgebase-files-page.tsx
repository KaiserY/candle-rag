import { useState, useEffect } from "react";

import { DataTable } from "@/components/knowledgebase-files-page/data-table";
import { columns } from "@/components/knowledgebase-files-page/data-table-columns";
import { File } from "@/schema";
import { openai } from "@/openai";

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
			<div className="container h-full py-6 overflow-hidden">
				<DataTable data={files} columns={columns} />
			</div>
		</>
	);
}
