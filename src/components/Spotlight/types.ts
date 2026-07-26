export type SpotlightView = 'search' | 'settings'

export interface SpotlightIndexJob {
  folderId: string
  folderName: string
  current: number
  total: number
  stage: 'discovering' | 'metadata' | 'queued' | 'embedding' | 'complete' | 'error'
  message: string
}
