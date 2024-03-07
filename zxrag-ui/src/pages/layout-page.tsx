import { Outlet } from "react-router-dom";

import { Navbar } from "@/components/navbar";

export function LayoutPage() {
	return (
		<>
			<div className="flex h-full flex-col">
				<Navbar className="border-b" />
				<Outlet />
			</div>
		</>
	);
}
