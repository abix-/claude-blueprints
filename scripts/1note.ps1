[CmdletBinding()]
param (
    [Parameter(Position = 0)]
    [ValidateSet('list', 'read', 'search')]
    [string]$Command,
    [Parameter(Position = 1, ValueFromRemainingArguments)]
    [string[]]$Rest
)

function Get-OneNoteApp {
    try {
        New-Object -ComObject OneNote.Application
    } catch {
        Write-Error "OneNote desktop app not available. Is it installed and not running as UWP?"
        exit 1
    }
}

function Get-1NoteList {
    [CmdletBinding()]
    param (
        [string]$SectionFilter
    )

    $OneNote = Get-OneNoteApp
    [xml]$Hierarchy = ""
    $OneNote.GetHierarchy("", [Microsoft.Office.InterOp.OneNote.HierarchyScope]::hsPages, [ref]$Hierarchy)

    foreach ($notebook in $Hierarchy.Notebooks.Notebook) {
        Write-Output "# $($notebook.name)"
        foreach ($section in $notebook.Section) {
            if ($SectionFilter -and $section.Name -ne $SectionFilter) { continue }
            Write-Output "  ## $($section.Name)"
            foreach ($page in $section.Page) {
                Write-Output "    - $($page.Name)"
            }
        }
        foreach ($sectionGroup in $notebook.SectionGroup) {
            if ($sectionGroup.Name -like "*_RecycleBin*") { continue }
            Write-Output "  ### $($sectionGroup.Name)"
            foreach ($section in $sectionGroup.Section) {
                if ($SectionFilter -and $section.Name -ne $SectionFilter) { continue }
                Write-Output "    ## $($section.Name)"
                foreach ($page in $section.Page) {
                    Write-Output "      - $($page.Name)"
                }
            }
        }
    }
}

function Get-1NoteRead {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory)]
        [string]$PageName,
        [string]$SectionName
    )

    $OneNote = Get-OneNoteApp
    [xml]$Hierarchy = ""
    $OneNote.GetHierarchy("", [Microsoft.Office.InterOp.OneNote.HierarchyScope]::hsPages, [ref]$Hierarchy)

    # Find pages
    if ($SectionName) {
        $sections = $Hierarchy.Notebooks.Notebook.Section | Where-Object { $_.Name -eq $SectionName }
        if (-not $sections) {
            # Check section groups too
            foreach ($notebook in $Hierarchy.Notebooks.Notebook) {
                foreach ($sg in $notebook.SectionGroup) {
                    $sections += $sg.Section | Where-Object { $_.Name -eq $SectionName }
                }
            }
        }
        $pages = $sections.Page | Where-Object { $_.Name -like "*$PageName*" }
    } else {
        $allPages = @()
        foreach ($notebook in $Hierarchy.Notebooks.Notebook) {
            foreach ($section in $notebook.Section) {
                foreach ($page in $section.Page) {
                    $allPages += $page
                }
            }
            foreach ($sg in $notebook.SectionGroup) {
                if ($sg.Name -like "*_RecycleBin*") { continue }
                foreach ($section in $sg.Section) {
                    foreach ($page in $section.Page) {
                        $allPages += $page
                    }
                }
            }
        }
        $pages = $allPages | Where-Object { $_.Name -like "*$PageName*" }
    }

    if (-not $pages) {
        Write-Error "Page '$PageName' not found"
        exit 1
    }

    foreach ($page in $pages) {
        [xml]$PageContent = ""
        $OneNote.GetPageContent($page.ID, [ref]$PageContent)

        Write-Output "# $($PageContent.Page.Name)"
        Write-Output ""

        foreach ($outline in $PageContent.Page.Outline) {
            foreach ($oe in $outline.OEChildren.OE) {
                # Text content
                if ($oe.T."#cdata-section") {
                    $text = $oe.T."#cdata-section" -replace '<[^>]+>', ''
                    Write-Output $text

                    # Child items (bullets/numbering)
                    foreach ($child in $oe.OEChildren.OE) {
                        $childText = $child.T."#cdata-section" -replace '<[^>]+>', ''
                        if ($child.List.Bullet) {
                            Write-Output "  - $childText"
                        } elseif ($child.List.Number) {
                            Write-Output "  $($child.List.Number.Text) $childText"
                        } else {
                            Write-Output "  $childText"
                        }
                    }
                }

                # Table content → markdown table
                if ($null -ne $oe.Table) {
                    $columns = @($oe.Table.Columns.Column)
                    $headers = @($oe.Table.Row[0].Cell.OEChildren.OE.T."#cdata-section" | ForEach-Object { $_ -replace '<[^>]+>', '' })

                    # Header row
                    $headerLine = "| " + ($headers -join " | ") + " |"
                    $separatorLine = "| " + (($headers | ForEach-Object { "---" }) -join " | ") + " |"
                    Write-Output ""
                    Write-Output $headerLine
                    Write-Output $separatorLine

                    # Data rows
                    foreach ($row in $oe.Table.Row | Select-Object -Skip 1) {
                        $cells = @()
                        for ($i = 0; $i -lt $columns.Count; $i++) {
                            $value = $row.Cell[$i].OEChildren.OE.T."#cdata-section" -replace '<[^>]+>', ''
                            $cells += $value
                        }
                        Write-Output "| $($cells -join ' | ') |"
                    }
                    Write-Output ""
                }
            }
        }
        Write-Output ""
    }
}

function Search-1Note {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory)]
        [string]$SearchTerm
    )

    $OneNote = Get-OneNoteApp
    [xml]$searchHierarchy = ""
    $OneNote.FindPages("", "$SearchTerm", [ref]$searchHierarchy)

    $notebooks = @()
    if ($searchHierarchy.Notebooks.UnfiledNotes) {
        $notebooks += $searchHierarchy.Notebooks.UnfiledNotes
    }
    if ($searchHierarchy.Notebooks.Notebook) {
        $notebooks += $searchHierarchy.Notebooks.Notebook
    }

    $results = @()
    foreach ($notebook in $notebooks) {
        foreach ($section in $notebook.Section) {
            foreach ($page in $section.Page) {
                $results += [PSCustomObject]@{
                    Page     = $page.Name
                    Path     = "$($notebook.Name) > $($section.Name)"
                    Modified = $page.lastModifiedTime
                }
            }
        }
        foreach ($sg in $notebook.SectionGroup) {
            foreach ($section in $sg.Section) {
                foreach ($page in $section.Page) {
                    $results += [PSCustomObject]@{
                        Page     = $page.Name
                        Path     = "$($notebook.Name) > $($sg.Name) > $($section.Name)"
                        Modified = $page.lastModifiedTime
                    }
                }
            }
        }
    }

    if ($results.Count -eq 0) {
        Write-Output "No results for '$SearchTerm'"
        return
    }

    $results = $results | Sort-Object Modified -Descending
    Write-Output "Found $($results.Count) results for '$SearchTerm':"
    Write-Output ""
    foreach ($r in $results) {
        Write-Output "  $($r.Page)"
        Write-Output "    $($r.Path) | $($r.Modified)"
    }
}

# Main dispatch
switch ($Command) {
    'list' {
        $section = $null
        for ($i = 0; $i -lt $Rest.Count; $i++) {
            if ($Rest[$i] -eq '-section' -and $i + 1 -lt $Rest.Count) {
                $section = $Rest[$i + 1]
            }
        }
        Get-1NoteList -SectionFilter $section
    }
    'read' {
        if (-not $Rest -or $Rest.Count -eq 0) {
            Write-Error "Usage: 1note.ps1 read <page name> [-section <section>]"
            exit 1
        }
        $pageName = $null
        $section = $null
        $collecting = @()
        for ($i = 0; $i -lt $Rest.Count; $i++) {
            if ($Rest[$i] -eq '-section' -and $i + 1 -lt $Rest.Count) {
                $section = $Rest[$i + 1]
                $i++
            } else {
                $collecting += $Rest[$i]
            }
        }
        $pageName = $collecting -join ' '
        Get-1NoteRead -PageName $pageName -SectionName $section
    }
    'search' {
        if (-not $Rest -or $Rest.Count -eq 0) {
            Write-Error "Usage: 1note.ps1 search <term>"
            exit 1
        }
        Search-1Note -SearchTerm ($Rest -join ' ')
    }
    default {
        Write-Output "Usage: 1note.ps1 <command> [args]"
        Write-Output ""
        Write-Output "Commands:"
        Write-Output "  list [-section <name>]              List notebooks, sections, pages"
        Write-Output "  read <page> [-section <name>]       Read page content as markdown"
        Write-Output "  search <term>                       Search across all notebooks"
        exit 1
    }
}
