import { onBeforeUnmount, onMounted, ref } from "vue";
import { imagyxApi } from "./api/tauri";
import { semanticRuntime } from "./services/semantic";

const TAG_CACHE_KEY = "imagyx.spotlight-top-tags.v1";
const FALLBACK_TAGS = [
  "woman",
  "portrait",
  "landscape",
  "dog",
  "cat",
  "car",
  "beach",
  "city",
  "night",
  "sunset",
];

function readCachedTags(): string[] {
  try {
    const parsed = JSON.parse(
      localStorage.getItem(TAG_CACHE_KEY) ?? "[]",
    ) as unknown;
    if (Array.isArray(parsed)) {
      const tags = parsed
        .filter(
          (tag): tag is string =>
            typeof tag === "string" && tag.trim().length > 0,
        )
        .slice(0, 10);
      if (tags.length) return tags;
    }
  } catch {
    /* cache facultatif */
  }
  return [...FALLBACK_TAGS];
}

export function capitalize(value: string): string {
  return value
    ? value.charAt(0).toLocaleUpperCase("fr") + value.slice(1)
    : value;
}

export function useTagTypewriter() {
  const initialTags = readCachedTags();
  const topTags = ref<string[]>(initialTags);
  const typedTag = ref<string>(initialTags[0] ?? FALLBACK_TAGS[0] ?? "image");

  let typewriterTimer: number | undefined;
  let tagIndex = 0;
  let characterIndex = typedTag.value.length;
  let deleting = false;

  async function warmTags() {
    try {
      await semanticRuntime.prewarmText();
      const concepts = await semanticRuntime.genericImageConcepts();
      const ranked = await imagyxApi.topImageTags(concepts, 10);
      if (ranked.length) {
        topTags.value = ranked;
        localStorage.setItem(TAG_CACHE_KEY, JSON.stringify(ranked));
        tagIndex %= ranked.length;
      }
    } catch {
      /* fallback conservé */
    }
  }

  function runTypewriter() {
    const tags = topTags.value.length ? topTags.value : FALLBACK_TAGS;
    const target = tags[tagIndex % tags.length] ?? "image";
    if (!deleting) {
      characterIndex = Math.min(target.length, characterIndex + 1);
      typedTag.value = target.slice(0, characterIndex);
      if (characterIndex >= target.length) {
        deleting = true;
        typewriterTimer = window.setTimeout(runTypewriter, 1250);
        return;
      }
      typewriterTimer = window.setTimeout(runTypewriter, 58);
      return;
    }
    characterIndex = Math.max(0, characterIndex - 1);
    typedTag.value = target.slice(0, characterIndex);
    if (characterIndex === 0) {
      deleting = false;
      tagIndex = (tagIndex + 1) % tags.length;
      typewriterTimer = window.setTimeout(runTypewriter, 180);
      return;
    }
    typewriterTimer = window.setTimeout(runTypewriter, 30);
  }

  onMounted(() => {
    void warmTags();
    typewriterTimer = window.setTimeout(runTypewriter, 320);
  });

  onBeforeUnmount(() => {
    if (typewriterTimer) window.clearTimeout(typewriterTimer);
  });

  return {
    topTags,
    typedTag,
  };
}
