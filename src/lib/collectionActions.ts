import ArrowCounterClockwise from "phosphor-svelte/lib/ArrowCounterClockwise";
import ImageSquare from "phosphor-svelte/lib/ImageSquare";
import PencilSimple from "phosphor-svelte/lib/PencilSimple";
import PushPin from "phosphor-svelte/lib/PushPin";
import PushPinSlash from "phosphor-svelte/lib/PushPinSlash";
import Trash from "phosphor-svelte/lib/Trash";
import { pickCover } from "$lib/cover";
import { format, i18n } from "$lib/i18n/index.svelte";
import { library } from "$lib/state/library.svelte";
import type { MenuItem } from "$lib/state/menu.svelte";
import { nav } from "$lib/state/nav.svelte";
import { toasts } from "$lib/state/toasts.svelte";
import { ui } from "$lib/state/ui.svelte";
import type { SavedCollection } from "$lib/types";

export async function changeCover(collection: SavedCollection): Promise<void> {
  try {
    const cover = await pickCover();
    if (cover) library.setCover(collection.id, cover);
  } catch {
    toasts.error(i18n.t.playlist.coverError);
  }
}

export async function deleteCollection(collection: SavedCollection): Promise<void> {
  // Imports can be brought back from their link; playlists made here cannot.
  if (collection.origin === "user") {
    const t = i18n.t.playlist;
    const confirmed = await ui.confirm(format(t.deleteTitle, { name: collection.name }), t.deleteBody, t.delete);
    if (!confirmed) return;
  }
  library.remove(collection.id);
  nav.forget((route) => route.name === "collection" && route.id === collection.id);
}

/** Actions offered for a playlist or album, from its context menu or its "more" button. */
export function collectionActions(collection: SavedCollection): MenuItem[] {
  const t = i18n.t.playlist;
  const items: MenuItem[] = [
    collection.pinned
      ? { label: t.unpin, icon: PushPinSlash, action: () => library.setPinned(collection.id, false) }
      : { label: t.pin, icon: PushPin, action: () => library.setPinned(collection.id, true) },
  ];
  if (collection.origin === "user") {
    items.push({
      label: t.rename,
      icon: PencilSimple,
      action: () => {
        nav.go({ name: "collection", id: collection.id });
        ui.renaming = collection.id;
      },
    });
  }
  items.push({ label: t.changeCover, icon: ImageSquare, action: () => void changeCover(collection) });
  if (collection.customArtwork) {
    items.push({
      label: t.resetCover,
      icon: ArrowCounterClockwise,
      action: () => library.setCover(collection.id, null),
    });
  }
  items.push({
    label: collection.origin === "user" ? t.delete : i18n.t.collection.remove,
    icon: Trash,
    danger: true,
    action: () => void deleteCollection(collection),
  });
  return items;
}
