/** Side of the stored cover; large enough for the 208px header at 2x scaling. */
const COVER_SIZE = 600;
const COVER_QUALITY = 0.86;

/**
 * Lets the user pick an image and returns it as a square JPEG data URL, center-cropped
 * and downscaled so a cover costs tens of kilobytes in the library, not megabytes.
 * Resolves to `null` when the picker is dismissed.
 */
export function pickCover(): Promise<string | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/png, image/jpeg, image/webp, image/gif";
    // WebKit does not dispatch the chooser result to a detached input.
    input.hidden = true;
    document.body.append(input);
    const finish = (value: string | null) => { input.remove(); resolve(value); };
    const fail = (error: unknown) => { input.remove(); reject(error); };
    input.addEventListener("cancel", () => finish(null), { once: true });
    input.addEventListener("change", () => {
      const file = input.files?.[0];
      if (!file) return finish(null);
      squareJpeg(file).then(finish, fail);
    });
    try { input.click(); } catch (error) { fail(error); }
  });
}

async function squareJpeg(file: Blob): Promise<string> {
  const image = await createImageBitmap(file);
  const side = Math.min(image.width, image.height);
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = Math.min(COVER_SIZE, side);
  const context = canvas.getContext("2d");
  if (!context) throw new Error("canvas unavailable");
  context.imageSmoothingQuality = "high";
  context.drawImage(
    image,
    (image.width - side) / 2,
    (image.height - side) / 2,
    side,
    side,
    0,
    0,
    canvas.width,
    canvas.height,
  );
  image.close();
  return canvas.toDataURL("image/jpeg", COVER_QUALITY);
}
