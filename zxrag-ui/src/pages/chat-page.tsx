import OpenAI from "openai";
import { CounterClockwiseClockIcon } from "@radix-ui/react-icons";

import { TemperatureSelector } from "@/components/temperature-selector";
import { MaxLengthSelector } from "@/components/maxlength-selector";
import { TopPSelector } from "@/components/top-p-selector";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";

export function ChatPage() {
	const openai = new OpenAI({
		baseURL: "/v1",
		apiKey: "NULL",
		dangerouslyAllowBrowser: true,
	});

	const handleClick = async () => {
		const stream = await openai.chat.completions.create({
			model: "gpt-4",
			messages: [{ role: "user", content: "Say this is a test" }],
			stream: true,
		});

		for await (const chunk of stream) {
			console.log(chunk.choices[0]?.delta?.content || "");
		}
	};

	return (
		<>
			<div className="container h-full py-6">
				<div className="grid h-full items-stretch gap-6 md:grid-cols-[1fr_200px]">
					<div className="md:order-1">
						<div className="flex flex-col space-y-4">
							<div className="grid h-full gap-6 lg:grid-cols-2">
								<div className="flex flex-col space-y-4">
									<div className="flex flex-1 flex-col space-y-2">
										<Label htmlFor="input">Input</Label>
										<Textarea
											id="input"
											placeholder="We is going to the market."
											className="flex-1 lg:min-h-[580px]"
										/>
									</div>
									<div className="flex flex-col space-y-2">
										<Label htmlFor="instructions">Instructions</Label>
										<Textarea
											id="instructions"
											placeholder="Fix the grammar."
										/>
									</div>
								</div>
								<div className="mt-[21px] min-h-[400px] rounded-md border bg-muted lg:min-h-[700px]" />
							</div>
							<div className="flex items-center space-x-2">
								<Button>Submit</Button>
								<Button variant="secondary">
									<span className="sr-only">Show history</span>
									<CounterClockwiseClockIcon className="h-4 w-4" />
								</Button>
							</div>
						</div>
					</div>
					<div className="flex-col space-y-4 sm:flex md:order-2">
						<TemperatureSelector defaultValue={[0.6]} />
						<MaxLengthSelector defaultValue={[256]} />
						<TopPSelector defaultValue={[0.9]} />
					</div>
				</div>
			</div>
		</>
	);
}
