import OpenAI from "openai";
import { CounterClockwiseClockIcon } from "@radix-ui/react-icons";
import { ChangeEvent, useState } from "react";

import { TemperatureSelector } from "@/components/temperature-selector";
import { MaxLengthSelector } from "@/components/maxlength-selector";
import { TopPSelector } from "@/components/top-p-selector";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";

interface ChatMessage {
	role: string;
	content: string;
}

export function ChatPage() {
	const [temperature, setTemperature] = useState<number[] | undefined>([0.6]);
	const [topP, setTopP] = useState<number[] | undefined>([0.9]);
	const [maxLength, setMaxLength] = useState<number[] | undefined>([128]);
	const [userPrompt, setUserPrompt] = useState("");
	const [output, setOutput] = useState("");
	const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);

	const openai = new OpenAI({
		baseURL: `${window.location.protocol}//${window.location.host}/v1`,
		apiKey: "NULL",
		dangerouslyAllowBrowser: true,
	});

	const handleUserPromptChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
		setUserPrompt(event.target.value);
	};

	const handleClick = async () => {
		const stream = await openai.chat.completions.create({
			model: "gpt-4",
			messages: [{ role: "user", content: userPrompt }],
			stream: true,
			max_tokens: maxLength === undefined ? undefined : maxLength[0],
			top_p: topP === undefined ? undefined : topP[0],
			temperature: temperature === undefined ? undefined : temperature[0],
		});

		for await (const chunk of stream) {
			const newContent = chunk.choices[0]?.delta?.content || "";

			console.log(newContent);

			setOutput((prev) => prev + newContent);
		}
	};

	return (
		<>
			<div className="container h-full py-6">
				<div className="grid h-full items-stretch gap-6 md:grid-cols-[1fr_200px]">
					<div className="flex flex-col space-y-4">
						<div className="flex flex-row gap-6 h-full lg:grid-cols-2">
							<div className="flex flex-col basis-1/3 space-y-4">
								<div className="flex flex-1 flex-col space-y-2">
									<Label htmlFor="input">Input</Label>
									<Textarea
										id="input"
										placeholder="We is going to the market."
										className="flex-1"
										value={userPrompt}
										onChange={handleUserPromptChange}
									/>
								</div>
								<div className="flex flex-col space-y-2">
									<Label htmlFor="instructions">Instructions</Label>
									<Textarea id="instructions" placeholder="Fix the grammar." />
								</div>
							</div>
							<div className="mt-[21px] basis-2/3">
								<div className="flex gap-3 w-full p-2">
									<Avatar className="h-6 w-6">
										<AvatarFallback>Y</AvatarFallback>
									</Avatar>
									<div className="flex flex-col">
										<span className="font-bold">You</span>
										<span>AIfasfjoasejfoisajfoasie</span>
									</div>
								</div>
                <div className="flex gap-3 w-full p-2">
									<Avatar className="h-6 w-6">
										<AvatarFallback className="bg-red-500">AI</AvatarFallback>
									</Avatar>
									<div className="flex flex-col">
										<span className="font-bold">You</span>
										<span>AIfasfjoasejfoisajfoasie</span>
									</div>
								</div>
							</div>
						</div>
						<div className="flex items-center space-x-2">
							<Button onClick={handleClick}>Submit</Button>
							<Button variant="secondary">
								<span className="sr-only">Show history</span>
								<CounterClockwiseClockIcon className="h-4 w-4" />
							</Button>
						</div>
					</div>
					<div className="flex-col space-y-4 sm:flex md:order-2">
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
