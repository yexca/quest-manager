# Regenerates the checked-in application icon using Windows System.Drawing.
Add-Type -AssemblyName System.Drawing
$questIconDirectory = Join-Path (Split-Path -Parent $PSScriptRoot) 'src-tauri\icons'
New-Item -ItemType Directory -Path $questIconDirectory -Force | Out-Null
$questBitmap = New-Object Drawing.Bitmap(256, 256)
$questGraphics = [Drawing.Graphics]::FromImage($questBitmap)
$questGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$questGraphics.Clear([Drawing.ColorTranslator]::FromHtml('#183b30'))
$questBrush = New-Object Drawing.SolidBrush([Drawing.ColorTranslator]::FromHtml('#e9f5ed'))
$questPath = New-Object Drawing.Drawing2D.GraphicsPath
$questPath.AddArc(35, 69, 56, 56, 180, 90)
$questPath.AddArc(165, 69, 56, 56, 270, 90)
$questPath.AddArc(165, 137, 56, 56, 0, 90)
$questPath.AddArc(35, 137, 56, 56, 90, 90)
$questPath.CloseFigure()
$questGraphics.FillPath($questBrush, $questPath)
$questDark = New-Object Drawing.SolidBrush([Drawing.ColorTranslator]::FromHtml('#183b30'))
$questGraphics.FillEllipse($questDark, 63, 108, 43, 43)
$questGraphics.FillEllipse($questDark, 151, 108, 43, 43)
$questGraphics.FillEllipse($questDark, 109, 166, 38, 40)
$questPngPath = Join-Path $questIconDirectory 'icon.png'
$questBitmap.Save($questPngPath, [Drawing.Imaging.ImageFormat]::Png)
$questPng = [IO.File]::ReadAllBytes($questPngPath)
$questStream = [IO.File]::Create((Join-Path $questIconDirectory 'icon.ico'))
$questWriter = New-Object IO.BinaryWriter($questStream)
$questWriter.Write([uint16]0); $questWriter.Write([uint16]1); $questWriter.Write([uint16]1)
$questWriter.Write([byte]0); $questWriter.Write([byte]0); $questWriter.Write([byte]0); $questWriter.Write([byte]0)
$questWriter.Write([uint16]1); $questWriter.Write([uint16]32); $questWriter.Write([uint32]$questPng.Length); $questWriter.Write([uint32]22)
$questWriter.Write($questPng)
$questWriter.Dispose(); $questGraphics.Dispose(); $questBitmap.Dispose(); $questPath.Dispose(); $questBrush.Dispose(); $questDark.Dispose()
