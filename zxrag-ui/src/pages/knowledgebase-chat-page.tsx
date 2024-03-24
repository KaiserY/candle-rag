import OpenAI from "openai";
import { ReloadIcon, TrashIcon } from "@radix-ui/react-icons";
import { ChangeEvent, useState, useEffect } from "react";

import { KnowledgeBase } from "@/schema";
import { TemperatureSelector } from "@/components/temperature-selector";
import { MaxLengthSelector } from "@/components/maxlength-selector";
import { TopPSelector } from "@/components/top-p-selector";
import { KnowledgeBaseSelector } from "@/components/knowledgebase-selector";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";

interface ChatMessage {
	role: string;
	content: string;
}

export function KnowledgebaseChatPage() {
	const [temperature, setTemperature] = useState<number[] | undefined>([0.6]);
	const [topP, setTopP] = useState<number[] | undefined>([0.9]);
	const [maxLength, setMaxLength] = useState<number[] | undefined>([128]);
	const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
	const [lastUserMessage, setLastUserMessage] = useState("");
	const [lastAssistantMessage, setLastAssistantMessage] = useState("");
	const [systemMessage, setSystemMessage] = useState("");
	const [isLoading, setIsLoading] = useState<boolean>(false);
	const [knowledgeBases, setKnowledgeBases] = useState<KnowledgeBase[]>([]);
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

	const handleUserMessageChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
		setLastUserMessage(event.target.value);
	};

	const handleSystemMessageChange = (
		event: ChangeEvent<HTMLTextAreaElement>,
	) => {
		setSystemMessage(event.target.value);
	};

	const handleClick = async () => {
		setIsLoading(true);

		const mergeChatMessages: ChatMessage[] = [];

		if (lastAssistantMessage !== "") {
			setChatMessages((chatMessages) => [
				...chatMessages,
				{ role: "assistant", content: lastAssistantMessage },
			]);

			mergeChatMessages.push({
				role: "assistant",
				content: lastAssistantMessage,
			});

			setLastAssistantMessage("");
		}

		setChatMessages((chatMessages) => [
			...chatMessages,
			{ role: "user", content: lastUserMessage },
		]);

		mergeChatMessages.push({ role: "user", content: lastUserMessage });

		setLastUserMessage("");

		try {
			const openai = new OpenAI({
				baseURL: `${window.location.protocol}//${window.location.host}/v1/knowledgebases/${selectedknowledgeBase.id}/`,
				apiKey: "NULL",
				dangerouslyAllowBrowser: true,
			});

			const stream = await openai.chat.completions.create({
				model: "NULL",
				messages: [
					...chatMessages.filter(
						(chat) => chat.role === "user" || chat.role === "assistant",
					),
					{ role: "user", content: lastUserMessage },
				].map((chat) => {
					if (chat.role === "user") {
						return { role: "user", content: chat.content };
					}

					return { role: "assistant", content: chat.content };
				}),
				stream: true,
				max_tokens: maxLength === undefined ? undefined : maxLength[0],
				top_p: topP === undefined ? undefined : topP[0],
				temperature: temperature === undefined ? undefined : temperature[0],
			});

			for await (const chunk of stream) {
				const newContent = chunk.choices[0]?.delta?.content || "";

				setLastAssistantMessage((prev) => prev + newContent);
			}
		} catch (error) {
			console.error(error);

			setIsLoading(false);

			return;
		}

		setIsLoading(false);
	};

	const handleClearHistroy = async () => {
		setLastUserMessage("");
		setLastAssistantMessage("");
		setSystemMessage("");
		setChatMessages([]);
	};

	return (
		<>
			<div className="container h-full py-6 overflow-hidden">
				<div className="flex h-full gap-6">
					<div className="flex flex-col basis-3/12 space-y-4">
						<div className="flex flex-1 flex-col space-y-2">
							<Label htmlFor="input">Input</Label>
							<Textarea
								id="input"
								placeholder="We is going to the market."
								className="flex-1"
								value={lastUserMessage}
								onChange={handleUserMessageChange}
								disabled={isLoading}
							/>
						</div>
						<div className="flex flex-col space-y-2">
							<Label htmlFor="instructions">Instructions</Label>
							<Textarea
								id="instructions"
								placeholder="Fix the grammar."
								value={systemMessage}
								onChange={handleSystemMessageChange}
								disabled={isLoading}
							/>
						</div>
						<div className="flex items-center justify-between space-x-2">
							<Button
								onClick={handleClick}
								disabled={isLoading || lastUserMessage === ""}
							>
								{isLoading && (
									<ReloadIcon className="mr-2 h-4 w-4 animate-spin" />
								)}{" "}
								Send
							</Button>
							<Button
								variant="secondary"
								disabled={isLoading}
								onClick={handleClearHistroy}
							>
								<span className="sr-only">Clear Histroy</span>
								{isLoading && (
									<ReloadIcon className="mr-2 h-4 w-4 animate-spin" />
								)}{" "}
								<TrashIcon className="h-4 w-4 text-red-500" />
							</Button>
						</div>
					</div>
					<div className="flex flex-col basis-7/12 max-h-full overflow-auto">
						{chatMessages.map((chat) => {
							if (chat.role === "user") {
								return (
									<div key={crypto.randomUUID()} className="flex gap-3 w-full p-2">
										<Avatar className="h-6 w-6">
											<AvatarFallback>Y</AvatarFallback>
										</Avatar>
										<div className="flex flex-col">
											<span className="font-bold">You</span>
											<span>{chat.content}</span>
										</div>
									</div>
								);
							}
							if (chat.role === "assistant") {
								return (
									<div key={crypto.randomUUID()} className="flex gap-3 w-full p-2">
										<Avatar className="h-6 w-6">
											<AvatarFallback className="bg-red-500 text-white">
												A
											</AvatarFallback>
										</Avatar>
										<div className="flex flex-col">
											<span className="font-bold">AI</span>
											<span>{chat.content}</span>
										</div>
									</div>
								);
							}
							return null;
						})}
						{lastAssistantMessage !== "" && (
							<div className="flex gap-3 w-full p-2">
								<Avatar className="h-6 w-6">
									<AvatarFallback className="bg-red-500 text-white">
										A
									</AvatarFallback>
								</Avatar>
								<div className="flex flex-col">
									<span className="font-bold">AI</span>
									<span>{lastAssistantMessage}</span>
								</div>
							</div>
						)}
					</div>
					<div className="flex flex-col basis-2/12 space-y-4">
						<KnowledgeBaseSelector
							knowledgeBases={knowledgeBases}
							selectedknowledgeBase={selectedknowledgeBase}
							setSelectedknowledgeBase={setSelectedknowledgeBase}
						/>
						<TemperatureSelector
							temperature={temperature}
							setTemperature={setTemperature}
						/>
						<MaxLengthSelector
							maxLength={maxLength}
							setMaxLength={setMaxLength}
						/>
						<TopPSelector topP={topP} setTopP={setTopP} />
					</div>
				</div>
			</div>
		</>
	);
}
