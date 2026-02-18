/**
 * Projects Page
 *
 * Full-page project management with:
 * - Project list with card layout
 * - Create new project functionality
 * - Delete project with confirmation
 * - Navigate to project sessions
 */

import * as React from "react"
import { useNavigate } from "@tanstack/react-router"
import { FolderKanban, Plus, Trash2, Folder, Clock, ExternalLink } from "lucide-react"

import { Button } from "@/components/ui/Button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card"
import { Input } from "@/components/ui/Input"
import { Label } from "@/components/ui/Label"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/Dialog"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/AlertDialog"
import { DirectoryPicker } from "@/components/shared/DirectoryPicker"
import { useProjectStore, useProjects, useProjectsLoading } from "@/stores/project"
import { useToast } from "@/hooks/use-toast"
import type { ProjectInfo } from "@/lib/types"
import { cn } from "@/lib/utils"

// ============================================================================
// Helper Functions
// ============================================================================

function formatDate(timestamp: number): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) {
    return "Today"
  } else if (days === 1) {
    return "Yesterday"
  } else if (days < 7) {
    return `${days} days ago`
  } else {
    return date.toLocaleDateString()
  }
}

function getProjectDisplayName(project: ProjectInfo): string {
  if (project.name) return project.name
  // Extract directory name from worktree path
  const parts = project.worktree.split("/").filter(Boolean)
  return parts[parts.length - 1] || project.worktree
}

// ============================================================================
// Empty State Component
// ============================================================================

interface ProjectsEmptyStateProps {
  onCreateProject: () => void
  isCreating: boolean
}

function ProjectsEmptyState({ onCreateProject, isCreating }: ProjectsEmptyStateProps) {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <Card className="max-w-md text-center">
        <CardContent className="pt-6 space-y-4">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
            <FolderKanban className="h-8 w-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-semibold">No projects yet</h3>
            <p className="text-sm text-muted-foreground">
              Create your first project to organize your coding sessions by directory.
            </p>
          </div>
          <Button onClick={onCreateProject} disabled={isCreating}>
            <Plus className="mr-2 h-4 w-4" />
            {isCreating ? "Creating..." : "New Project"}
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

// ============================================================================
// Projects Header Component
// ============================================================================

interface ProjectsHeaderProps {
  totalProjects: number
  onCreateProject: () => void
  isCreating: boolean
}

function ProjectsHeader({ totalProjects, onCreateProject, isCreating }: ProjectsHeaderProps) {
  return (
    <div className="flex items-center justify-between p-4 border-b bg-muted/30">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <FolderKanban className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold">Projects</h1>
          <p className="text-sm text-muted-foreground">
            {totalProjects === 0
              ? "No projects"
              : `${totalProjects} project${totalProjects !== 1 ? "s" : ""}`}
          </p>
        </div>
      </div>
      <Button onClick={onCreateProject} disabled={isCreating} data-testid="create-project-btn">
        <Plus className="mr-2 h-4 w-4" />
        {isCreating ? "Creating..." : "New Project"}
      </Button>
    </div>
  )
}

// ============================================================================
// Project Card Component
// ============================================================================

interface ProjectCardProps {
  project: ProjectInfo
  onDelete: (id: string) => void
  isDeleting: boolean
}

function ProjectCard({ project, onDelete, isDeleting }: ProjectCardProps) {
  const navigate = useNavigate()
  const displayName = getProjectDisplayName(project)

  const handleViewSessions = () => {
    navigate({ to: "/sessions", search: { projectId: project.id } })
  }

  return (
    <Card className={cn("transition-all hover:shadow-md", isDeleting && "opacity-50")}>
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div
              className="flex h-8 w-8 items-center justify-center rounded-md"
              style={{ backgroundColor: project.icon?.color ?? "hsl(var(--primary) / 0.1)" }}
            >
              {project.icon?.url ? (
                <img
                  src={project.icon.url}
                  alt=""
                  className="h-5 w-5 rounded"
                />
              ) : (
                <Folder className="h-4 w-4 text-primary" />
              )}
            </div>
            <CardTitle className="text-base">{displayName}</CardTitle>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-muted-foreground hover:text-destructive"
            onClick={() => onDelete(project.id)}
            disabled={isDeleting || project.id === "global"}
            title={project.id === "global" ? "Cannot delete global project" : "Delete project"}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
        <CardDescription className="text-xs truncate" title={project.worktree}>
          {project.worktree}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-2">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            <span>{formatDate(project.time.updated)}</span>
          </div>
          {project.vcs === "git" && (
            <span className="px-1.5 py-0.5 bg-muted rounded text-[10px] font-medium">
              git
            </span>
          )}
        </div>
        <Button
          variant="outline"
          size="sm"
          className="w-full mt-3"
          onClick={handleViewSessions}
        >
          <ExternalLink className="mr-2 h-3 w-3" />
          View Sessions
        </Button>
      </CardContent>
    </Card>
  )
}

// ============================================================================
// Main Projects Page Component
// ============================================================================

export function Projects() {
  const projects = useProjects()
  const { isLoaded, isCreating } = useProjectsLoading()
  const { loadProjects, createProject, deleteProject } = useProjectStore()
  const { toast } = useToast()

  // Create dialog state
  const [createDialogOpen, setCreateDialogOpen] = React.useState(false)
  const [directoryPickerOpen, setDirectoryPickerOpen] = React.useState(false)
  const [selectedDirectory, setSelectedDirectory] = React.useState("")
  const [projectName, setProjectName] = React.useState("")

  // Delete dialog state
  const [deleteDialogOpen, setDeleteDialogOpen] = React.useState(false)
  const [projectToDelete, setProjectToDelete] = React.useState<string | null>(null)
  const [isDeleting, setIsDeleting] = React.useState(false)

  // Load projects on mount
  React.useEffect(() => {
    loadProjects()
  }, [loadProjects])

  // Open create dialog
  const handleOpenCreateDialog = () => {
    setSelectedDirectory("")
    setProjectName("")
    setCreateDialogOpen(true)
  }

  // Handle directory selection from picker
  const handleDirectorySelect = (path: string) => {
    setSelectedDirectory(path)
    // Auto-fill project name from directory name
    const dirName = path.split("/").filter(Boolean).pop() ?? ""
    setProjectName(dirName)
  }

  // Create project
  const handleCreateProject = async () => {
    if (!selectedDirectory) {
      toast({
        title: "Directory required",
        description: "Please select a directory for the project.",
        variant: "destructive",
      })
      return
    }

    try {
      await createProject({
        directory: selectedDirectory,
        name: projectName || undefined,
      })
      setCreateDialogOpen(false)
      toast({
        title: "Project created",
        description: "Your new project is ready.",
      })
    } catch {
      toast({
        title: "Failed to create project",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    }
  }

  // Open delete confirmation
  const handleDeleteClick = (projectId: string) => {
    setProjectToDelete(projectId)
    setDeleteDialogOpen(true)
  }

  // Confirm delete
  const handleConfirmDelete = async () => {
    if (!projectToDelete) return

    setIsDeleting(true)
    try {
      await deleteProject(projectToDelete)
      toast({
        title: "Project deleted",
        description: "The project has been removed.",
      })
    } catch {
      toast({
        title: "Failed to delete project",
        description: "An error occurred. Please try again.",
        variant: "destructive",
      })
    } finally {
      setIsDeleting(false)
      setDeleteDialogOpen(false)
      setProjectToDelete(null)
    }
  }

  // Show empty state if no projects after loading
  if (isLoaded && projects.length === 0) {
    return (
      <div className="flex flex-col h-full bg-background">
        <ProjectsHeader
          totalProjects={0}
          onCreateProject={handleOpenCreateDialog}
          isCreating={isCreating}
        />
        <ProjectsEmptyState onCreateProject={handleOpenCreateDialog} isCreating={isCreating} />

        {/* Create Project Dialog */}
        <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create New Project</DialogTitle>
              <DialogDescription>
                Select a directory to create a new project. Projects help you organize sessions by codebase.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label>Directory</Label>
                <div className="flex gap-2">
                  <Input
                    value={selectedDirectory}
                    placeholder="Select a directory..."
                    readOnly
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={() => setDirectoryPickerOpen(true)}>
                    Browse
                  </Button>
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="project-name">Project Name (optional)</Label>
                <Input
                  id="project-name"
                  value={projectName}
                  onChange={(e) => setProjectName(e.target.value)}
                  placeholder="My Project"
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
                Cancel
              </Button>
              <Button onClick={handleCreateProject} disabled={!selectedDirectory || isCreating}>
                {isCreating ? "Creating..." : "Create Project"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Directory Picker */}
        <DirectoryPicker
          open={directoryPickerOpen}
          onOpenChange={setDirectoryPickerOpen}
          onSelect={handleDirectorySelect}
        />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full bg-background">
      <ProjectsHeader
        totalProjects={projects.length}
        onCreateProject={handleOpenCreateDialog}
        isCreating={isCreating}
      />

      {/* Project Grid */}
      <div className="flex-1 overflow-auto p-4">
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {projects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              onDelete={handleDeleteClick}
              isDeleting={projectToDelete === project.id && isDeleting}
            />
          ))}
        </div>
      </div>

      {/* Create Project Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Project</DialogTitle>
            <DialogDescription>
              Select a directory to create a new project. Projects help you organize sessions by codebase.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Directory</Label>
              <div className="flex gap-2">
                <Input
                  value={selectedDirectory}
                  placeholder="Select a directory..."
                  readOnly
                  className="flex-1"
                />
                <Button variant="outline" onClick={() => setDirectoryPickerOpen(true)}>
                  Browse
                </Button>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="project-name">Project Name (optional)</Label>
              <Input
                id="project-name"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                placeholder="My Project"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreateProject} disabled={!selectedDirectory || isCreating}>
              {isCreating ? "Creating..." : "Create Project"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Directory Picker */}
      <DirectoryPicker
        open={directoryPickerOpen}
        onOpenChange={setDirectoryPickerOpen}
        onSelect={handleDirectorySelect}
      />

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Project?</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove the project from the list. Your files and sessions will not be deleted.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isDeleting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleConfirmDelete}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isDeleting ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

export default Projects
