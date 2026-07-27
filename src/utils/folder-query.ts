import type { FollowedFolder } from '../types'

export interface FolderQuery {
  folder: FollowedFolder | null
  query: string
  isFolderPrefix: boolean
  folderText: string
}

export function parseFolderQuery(value: string, folders: FollowedFolder[]): FolderQuery {
  if (!value.startsWith('!')) {
    return { folder: null, query: value.trim(), isFolderPrefix: false, folderText: '' }
  }

  const separator = value.indexOf(':')
  const folderText = value.slice(1, separator < 0 ? undefined : separator).trim()
  if (separator < 0) {
    return { folder: null, query: '', isFolderPrefix: true, folderText }
  }

  const normalized = folderText.toLocaleLowerCase('fr')
  const folder = folders.find((item) => item.name.toLocaleLowerCase('fr') === normalized) ?? null
  return {
    folder,
    query: folder ? value.slice(separator + 1).trim() : value.trim(),
    isFolderPrefix: false,
    folderText,
  }
}

export function folderQuerySuggestions(
  value: string,
  folders: FollowedFolder[],
  limit = 5,
): FollowedFolder[] {
  const parsed = parseFolderQuery(value, folders)
  if (!parsed.isFolderPrefix) return []
  const query = parsed.folderText.toLocaleLowerCase('fr')
  return folders
    .filter((folder) => folder.name.toLocaleLowerCase('fr').includes(query))
    .slice(0, limit)
}

export function formatFolderQuery(folder: FollowedFolder): string {
  return `!${folder.name}: `
}
