import { Navbar } from "../components/navbar";

export function ChatPage() {
	return (
		<>
			<div className="flex h-full flex-col py-6">
				<Navbar />
				<div className="divider px-4" />
				<div className="flex fllx-row gap-6 h-full px-4">
					<div className="flex flex-col grow">
						<div className="flex flex-row gap-6 h-full w-full p-4">
							<textarea
								placeholder="Bio"
								className="basis-1/2 textarea textarea-bordered rounded-md text-sm shadow-sm h-full resize-none"
							/>
							<div className="basis-1/2 rounded-md border px-4 py-2">Bio</div>
						</div>
						<div className="flex items-center">
							<button type="button" className="btn btn-neutral">
								Submit
							</button>
						</div>
					</div>
					<div className="flex flex-col space-y-4">
						<div className="grid gap-2">
							<div className="flex items-center justify-between">
								<label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
									Temperature
								</label>
								<span className="w-12 rounded-md border border-transparent px-2 py-0.5 text-right text-sm text-muted-foreground hover:border-border">
									0.56
								</span>
							</div>
							<input
								type="range"
								min={0}
								max="100"
								value="40"
								className="range range-xs"
							/>
						</div>
						<div className="grid gap-2">
							<div className="flex items-center justify-between">
								<label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
									Maximum Length
								</label>
								<span className="w-12 rounded-md border border-transparent px-2 py-0.5 text-right text-sm text-muted-foreground hover:border-border">
									0.56
								</span>
							</div>
							<input
								type="range"
								min={0}
								max="100"
								value="40"
								className="range range-xs"
							/>
						</div>
						<div className="grid gap-2">
							<div className="flex items-center justify-between">
								<label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
									Top P
								</label>
								<span className="w-12 rounded-md border border-transparent px-2 py-0.5 text-right text-sm text-muted-foreground hover:border-border">
									0.56
								</span>
							</div>
							<input
								type="range"
								min={0}
								max="100"
								value="40"
								className="range range-xs"
							/>
						</div>
					</div>
				</div>
			</div>
		</>
	);
}
