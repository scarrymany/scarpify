/** UI capabilities only; native permissions remain enforced by Tauri. */
export function capabilities(userAgent: string) {
  const android = /Android/i.test(userAgent);
  return { android, windowControls: !android, discordPresence: !android, outputSelection: !android };
}

export const platform = capabilities(navigator.userAgent);
