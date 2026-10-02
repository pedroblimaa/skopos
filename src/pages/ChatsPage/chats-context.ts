import { createContext } from "react";
import type { useChatCache } from "./useChatCache";

export const ChatsContext = createContext<ReturnType<typeof useChatCache> | null>(null);
