import {
	Link,
	Route,
	RouterProvider,
	createBrowserRouter,
	createRoutesFromElements,
} from "react-router-dom";

import { LayoutPage } from "@/pages/layout-page";
import { ChatPage } from "@/pages/chat-page";
import { KnowledgebasePage } from "@/pages/knowledgebase-page";
import { OCRPage } from "@/pages/ocr-page";
import { Toaster } from "@/components/ui/toaster";
import { ThemeProvider } from "@/components/theme-provider";

import "./App.css";

export default function App() {
	const router = createBrowserRouter(
		createRoutesFromElements(
			<Route path="/" element={<LayoutPage />}>
				<Route index element={<ChatPage />} />
				<Route path="chat" element={<ChatPage />} />
				<Route path="knowledgebase" element={<KnowledgebasePage />} />
				<Route path="ocr" element={<OCRPage />} />
				<Route path="*" element={<NoMatch />} />
			</Route>,
		),
	);

	return (
		<ThemeProvider defaultTheme="light" storageKey="vite-ui-theme">
			<RouterProvider router={router} />
			<Toaster />
		</ThemeProvider>
	);
}

function NoMatch() {
	return (
		<div>
			<h2>Nothing to see here!</h2>
			<p>
				<Link to="/">Go to the home page</Link>
			</p>
		</div>
	);
}
