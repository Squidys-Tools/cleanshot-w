#Requires -Version 7
# Generates the mymind benchmark corpus (images, screenshots, PDFs).
#
# Every file produced by this script is original and generated locally with
# .NET System.Drawing. Nothing is downloaded and nothing is copyrighted.
# Run from the repository root or from anywhere; output lands under
# benchmarks/corpus. Re-running is safe (idempotent overwrites).

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Corpus = Join-Path $Root 'corpus'
$ImgDir = Join-Path $Corpus 'images'
$ScrDir = Join-Path $Corpus 'screenshots'
$PdfDir = Join-Path $Corpus 'pdfs'
$ArtDir = Join-Path $Corpus 'articles'
$VidDir = Join-Path $Corpus 'videos'
$NoteDir = Join-Path $Corpus 'notes'
$ExpDir = Join-Path $Root 'expected'
$OcrExpDir = Join-Path $ExpDir 'ocr'

foreach ($dir in @($Corpus, $ImgDir, $ScrDir, $PdfDir, $ArtDir, $VidDir, $NoteDir, $ExpDir, $OcrExpDir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

$pngFormat = [System.Drawing.Imaging.ImageFormat]::Png

function C {
    param([byte]$r, [byte]$g, [byte]$b, [byte]$a = 255)
    [System.Drawing.Color]::FromArgb($a, $r, $g, $b)
}

function New-Canvas {
    param([int]$W, [int]$H, [System.Drawing.Color]$Back)
    $bmp = [System.Drawing.Bitmap]::new($W, $H)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear($Back)
    , @($bmp, $g)
}

function Save-Jpeg {
    param([System.Drawing.Bitmap]$Bmp, [string]$Path, [int]$Quality = 90)
    $codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() |
        Where-Object { $_.MimeType -eq 'image/jpeg' }
    $enc = [System.Drawing.Imaging.Encoder]::Quality
    $params = [System.Drawing.Imaging.EncoderParameters]::new(1)
    $params.Param[0] = [System.Drawing.Imaging.EncoderParameter]::new($enc, $Quality)
    $Bmp.Save($Path, $codec, $params)
}

function Save-Png {
    param([System.Drawing.Bitmap]$Bmp, [string]$Path)
    $Bmp.Save($Path, $pngFormat)
}

function Dispose-Canvas {
    param($Canvas)
    $Canvas[1].Dispose()
    $Canvas[0].Dispose()
}

function New-Font {
    param([string]$Family, [single]$Size, [System.Drawing.FontStyle]$Style = 'Regular')
    [System.Drawing.Font]::new($Family, $Size, $Style, [System.Drawing.GraphicsUnit]::Pixel)
}

function Brush {
    param([System.Drawing.Color]$Color)
    [System.Drawing.SolidBrush]::new($Color)
}

function Draw-WindowChrome {
    # Draws a light browser/app title bar so images read as screenshots.
    param(
        [System.Drawing.Graphics]$g,
        [int]$W,
        [string]$Url = ''
    )
    $g.FillRectangle((Brush (C 236 236 233)), 0, 0, $W, 40)
    foreach ($x in @(16, 34, 52)) {
        $g.FillEllipse((Brush (C 226 226 220)), $x, 15, 10, 10)
    }
    if ($Url) {
        $g.FillRectangle((Brush (C 255 255 255)), 80, 10, $W - 180, 20)
        $g.DrawString(
            $Url,
            (New-Font 'Segoe UI' 12),
            (Brush (C 120 120 118)),
            92, 12)
    }
}

# ---------------------------------------------------------------- images --

function Write-Photo {
    # A photographic-style landscape: gradient sky, sun, mountains, trees.
    $W = 1024; $H = 768
    $canvas = New-Canvas $W $H (C 150 190 220)
    $g = $canvas[1]

    $sky = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.RectangleF]::new(0, 0, $W, $H), (C 196 222 240), (C 242 214 178), 90)
    $g.FillRectangle($sky, 0, 0, $W, $H)

    $g.FillEllipse((Brush (C 255 248 222)), 640, 90, 150, 150)

    $farMtn = [System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new(0, 560),
        [System.Drawing.PointF]::new(160, 380),
        [System.Drawing.PointF]::new(330, 520),
        [System.Drawing.PointF]::new(520, 350),
        [System.Drawing.PointF]::new(760, 540),
        [System.Drawing.PointF]::new($W, 420),
        [System.Drawing.PointF]::new($W, 600),
        [System.Drawing.PointF]::new(0, 600)
    )
    $g.FillPolygon((Brush (C 138 150 158)), $farMtn)

    $nearMtn = [System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new(0, 640),
        [System.Drawing.PointF]::new(240, 500),
        [System.Drawing.PointF]::new(430, 610),
        [System.Drawing.PointF]::new(700, 470),
        [System.Drawing.PointF]::new($W, 600),
        [System.Drawing.PointF]::new($W, 660),
        [System.Drawing.PointF]::new(0, 660)
    )
    $g.FillPolygon((Brush (C 96 112 104)), $nearMtn)

    $tree = Brush (C 60 84 66)
    $g.FillPolygon($tree, @(
        [System.Drawing.PointF]::new(120, 700),
        [System.Drawing.PointF]::new(155, 610),
        [System.Drawing.PointF]::new(190, 700)))
    $g.FillPolygon($tree, @(
        [System.Drawing.PointF]::new(165, 660),
        [System.Drawing.PointF]::new(195, 585),
        [System.Drawing.PointF]::new(225, 660)))

    $g.FillRectangle((Brush (C 84 120 96)), 0, 690, $W, 78)

    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-photo-01.jpg')
    Dispose-Canvas $canvas
}

function Write-DesignRef {
    # A pastel moodboard: swatch grid plus translucent overlapping shapes.
    $W = 1024; $H = 768
    $canvas = New-Canvas $W $H (C 247 247 245)
    $g = $canvas[1]

    $swatches = @(
        (C 239 210 190), (C 196 218 232), (C 210 226 196),
        (C 238 228 174), (C 226 206 232), (C 240 190 176)
    )
    $sw = 240; $sh = 180
    for ($row = 0; $row -lt 2; $row++) {
        for ($col = 0; $col -lt 3; $col++) {
            $x = 64 + $col * 296; $y = 90 + $row * 296
            $g.FillRectangle((Brush $swatches[$row * 3 + $col]), $x, $y, $sw, $sh)
        }
    }

    $g.FillEllipse((Brush (C 120 60 40)), 430, 150, 180, 180)
    $g.FillEllipse((Brush (C 60 90 70)), 560, 260, 220, 220)

    $g.DrawString(
        'palette study no. 4',
        (New-Font 'Georgia' 30 ([System.Drawing.FontStyle]::Italic)),
        (Brush (C 70 70 70)),
        64, 30)

    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-design-ref-01.jpg')
    Dispose-Canvas $canvas
}

function Draw-RoomScene {
    param([System.Drawing.Graphics]$g, [int]$W, [int]$H, [string]$Tone)
    $warm = $Tone -eq 'warm'
    $wall = if ($warm) { C 214 184 158 } else { C 201 190 179 }
    $g.Clear($wall)

    $floor = if ($warm) { C 150 110 84 } else { C 146 128 112 }
    $g.FillRectangle((Brush $floor), 0, [int]($H * 0.72), $W, [int]($H * 0.28))

    $g.FillRectangle((Brush (C 225 234 240)), 80, 55, 300, 235)
    $sun = if ($warm) { C 244 198 130 } else { C 216 206 192 }
    $g.FillEllipse((Brush $sun), 330, 90, 90, 90)

    $g.FillRectangle((Brush (C 120 96 72)), 520, 400, 84, 96)
    $leaf = Brush (C 96 140 100)
    $g.FillEllipse($leaf, 500, 340, 60, 80)
    $g.FillEllipse($leaf, 562, 330, 70, 92)
    $g.FillEllipse($leaf, 522, 312, 50, 62)

    $chair = Brush (C 90 70 55)
    $g.FillRectangle($chair, 185, 460, 120, 22)
    $g.FillRectangle($chair, 205, 372, 16, 112)
    $g.FillRectangle($chair, 269, 372, 16, 112)
    $g.FillRectangle($chair, 185, 372, 100, 16)

    $rug = if ($warm) { C 188 120 92 } else { C 168 148 128 }
    $g.FillEllipse((Brush $rug), 420, 540, 200, 62)
}

function Write-SimilarA {
    $W = 1024; $H = 768
    $canvas = New-Canvas $W $H (C 214 184 158)
    Draw-RoomScene $canvas[1] $W $H 'warm'
    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-similar-01.jpg')
    Dispose-Canvas $canvas

    $canvas = New-Canvas $W $H (C 201 190 179)
    Draw-RoomScene $canvas[1] $W $H 'cool'
    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-similar-02.jpg')
    Dispose-Canvas $canvas
}

function Draw-Dunes {
    param([System.Drawing.Graphics]$g, [int]$W, [int]$H, [string]$Tone)
    $orange = $Tone -eq 'orange'
    $skyTop = if ($orange) { C 255 214 150 } else { C 160 150 210 }
    $skyBottom = if ($orange) { C 250 150 90 } else { C 80 66 130 }
    $sky = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.RectangleF]::new(0, 0, $W, $H), $skyTop, $skyBottom, 90)
    $g.FillRectangle($sky, 0, 0, $W, $H)

    $g.FillEllipse((Brush (C 255 244 214)), 420, 120, 180, 180)

    $duneFar = if ($orange) { C 214 128 76 } else { C 90 74 128 }
    $g.FillPolygon((Brush $duneFar), @(
        [System.Drawing.PointF]::new(0, 470),
        [System.Drawing.PointF]::new(180, 350),
        [System.Drawing.PointF]::new(380, 460),
        [System.Drawing.PointF]::new(560, 330),
        [System.Drawing.PointF]::new($W, 450),
        [System.Drawing.PointF]::new($W, $H),
        [System.Drawing.PointF]::new(0, $H)))

    $duneNear = if ($orange) { C 168 94 52 } else { C 58 46 96 }
    $g.FillPolygon((Brush $duneNear), @(
        [System.Drawing.PointF]::new(0, 620),
        [System.Drawing.PointF]::new(240, 500),
        [System.Drawing.PointF]::new(520, 610),
        [System.Drawing.PointF]::new(760, 480),
        [System.Drawing.PointF]::new($W, 580),
        [System.Drawing.PointF]::new($W, $H),
        [System.Drawing.PointF]::new(0, $H)))
}

function Write-SimilarB {
    $W = 1024; $H = 768
    $canvas = New-Canvas $W $H (C 255 214 150)
    Draw-Dunes $canvas[1] $W $H 'orange'
    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-similar-03.jpg')
    Dispose-Canvas $canvas

    $canvas = New-Canvas $W $H (C 160 150 210)
    Draw-Dunes $canvas[1] $W $H 'violet'
    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-similar-04.jpg')
    Dispose-Canvas $canvas
}

function Write-Distractor {
    # A dark blue ocean scene: visually different from both similarity groups.
    $W = 1024; $H = 768
    $canvas = New-Canvas $W $H (C 22 42 74)
    $g = $canvas[1]

    $water = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.RectangleF]::new(0, 0, $W, $H), (C 26 56 96), (C 10 20 44), 90)
    $g.FillRectangle($water, 0, 0, $W, $H)

    $g.FillEllipse((Brush (C 238 244 250)), 120, 90, 100, 100)

    $line = [System.Drawing.Pen]::new((C 190 210 230 120))
    $line.Width = 2
    foreach ($y in @(280, 360, 440, 520)) {
        $g.DrawBezier($line,
            [System.Drawing.PointF]::new(0, $y),
            [System.Drawing.PointF]::new($W * 0.33, $y - 40),
            [System.Drawing.PointF]::new($W * 0.66, $y + 40),
            [System.Drawing.PointF]::new($W, $y))
    }

    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-distractor-01.jpg')
    Dispose-Canvas $canvas
}

function Write-LowRes {
    # A deliberately tiny scene, kept at low resolution on purpose.
    $W = 120; $H = 90
    $canvas = New-Canvas $W $H (C 226 218 208)
    $g = $canvas[1]

    $g.FillRectangle((Brush (C 148 112 84)), 10, 60, 100, 26)
    $g.FillEllipse((Brush (C 190 52 44)), 45, 22, 40, 38)
    $g.FillRectangle((Brush (C 96 82 64)), 60, 8, 6, 30)

    Save-Jpeg $canvas[0] (Join-Path $ImgDir 'image-lowres-01.jpg')
    Dispose-Canvas $canvas
}

function Write-Meme {
    $W = 900; $H = 900
    $canvas = New-Canvas $W $H (C 250 220 110)
    $g = $canvas[1]

    $g.FillEllipse((Brush (C 255 244 214)), 250, 330, 400, 400)
    $g.FillEllipse((Brush (C 60 60 60)), 340, 470, 28, 44)
    $g.FillEllipse((Brush (C 60 60 60)), 530, 470, 28, 44)
    $g.FillEllipse((Brush (C 200 120 100)), 380, 560, 140, 60)

    $topText = 'ME: I WILL JUST SAVE ONE LINK'
    $bottomText = 'ME 10 SECONDS LATER: ANOTHER 47 TABS'
    $brush = Brush (C 20 20 20)

    foreach ($entry in @(
            @{ Text = $topText; Y = 40 },
            @{ Text = $bottomText; Y = 800 }
        )) {
        $font = New-Font 'Impact' 52 ([System.Drawing.FontStyle]::Bold)
        $size = $g.MeasureString($entry.Text, $font)
        $x = ($W - $size.Width) / 2
        $g.DrawString($entry.Text, $font, $brush, $x - 3, $entry.Y + 3)
        $g.DrawString($entry.Text, $font, $brush, $x + 3, $entry.Y - 3)
        $g.DrawString($entry.Text, $font, $brush, $x, $entry.Y)
        $font.Dispose()
    }

    Save-Png $canvas[0] (Join-Path $ImgDir 'image-meme-01.png')
    Dispose-Canvas $canvas
    Write-ExpectedOcr 'image-meme-01' ($topText + "`n" + $bottomText)
}

$handwritingText = @(
    'friday',
    'call the studio',
    'finish the reading list',
    'water the plants',
    'remember: slow is fine'
)

function Write-Handwriting {
    # Lined paper with a Comic Sans scrawl that stands in for handwriting.
    $W = 900; $H = 560
    $canvas = New-Canvas $W $H (C 250 249 244)
    $g = $canvas[1]

    $rule = [System.Drawing.Pen]::new((C 188 208 226))
    $rule.Width = 1
    for ($y = 90; $y -lt $H - 40; $y += 44) {
        $g.DrawLine($rule, 70, $y, $W - 70, $y)
    }

    $font = New-Font 'Comic Sans MS' 26
    $ink = Brush (C 36 54 96)
    $y = 100
    foreach ($lineText in $handwritingText) {
        $g.TranslateTransform(84, $y + 8)
        $g.RotateTransform(-1.6)
        $g.DrawString($lineText, $font, $ink, 0, 0)
        $g.ResetTransform()
        $y += 44
    }

    Save-Png $canvas[0] (Join-Path $ImgDir 'image-handwriting-01.png')
    Dispose-Canvas $canvas
    Write-ExpectedOcr 'image-handwriting-01' ($handwritingText -join "`n")
}

$rotatedHeadline = 'FIELD REPORT'
$rotatedBody = 'The expedition set out before dawn with three cameras, a paper map, and no signal. By noon the fog had lifted and the valley opened like a held breath.'

function Write-RotatedText {
    $W = 900; $H = 620
    $canvas = New-Canvas $W $H (C 252 250 246)
    $g = $canvas[1]

    $g.TranslateTransform(420, 240)
    $g.RotateTransform(14)

    $g.DrawString(
        $rotatedHeadline,
        (New-Font 'Arial' 44 ([System.Drawing.FontStyle]::Bold)),
        (Brush (C 40 40 40)),
        [System.Drawing.RectangleF]::new(-320, -260, 640, 60))

    $g.DrawString(
        $rotatedBody,
        (New-Font 'Georgia' 26),
        (Brush (C 60 60 60)),
        [System.Drawing.RectangleF]::new(-320, -170, 620, 200))
    $g.ResetTransform()

    $g.DrawString(
        'Appendix B - rotation tolerance check',
        (New-Font 'Consolas' 14),
        (Brush (C 140 140 138)),
        40, 560)

    Save-Png $canvas[0] (Join-Path $ImgDir 'image-rotated-01.png')
    Dispose-Canvas $canvas
    Write-ExpectedOcr 'image-rotated-01' ($rotatedHeadline + "`n" + $rotatedBody + "`nAppendix B - rotation tolerance check")
}

$columnsText = @{
    left  = 'Barefoot in the morning grass. The kettle argues with itself. A stack of postcards, none of them addressed yet. The garden is patient. Rain on the tin roof sounds like applause you can keep.'
    right = 'Libraries smell like paper and rain. Somebody shelved the dictionaries out of order on purpose. Notes multiply in the margins overnight. A bookmark is a promise between two afternoons.'
}

function Write-Columns {
    $W = 900; $H = 620
    $canvas = New-Canvas $W $H (C 247 246 242)
    $g = $canvas[1]

    $g.DrawString(
        'FIELD NOTES - PAGE 12',
        (New-Font 'Georgia' 30),
        (Brush (C 40 40 40)),
        60, 36)

    $g.DrawLine([System.Drawing.Pen]::new((C 200 200 195)), 60, 84, 840, 84)

    $colWidth = 360
    $colIndex = 0
    foreach ($col in @('left', 'right')) {
        $x = 60 + $colIndex * 420
        $g.DrawString(
            ($col.ToUpper()),
            (New-Font 'Arial' 16 ([System.Drawing.FontStyle]::Bold)),
            (Brush (C 90 90 90)),
            $x, 110)
        $g.DrawString(
            $columnsText[$col],
            (New-Font 'Georgia' 20),
            (Brush (C 55 55 55)),
            [System.Drawing.RectangleF]::new($x, 150, $colWidth, 420))
        $colIndex++
    }

    Save-Png $canvas[0] (Join-Path $ImgDir 'image-columns-01.png')
    Dispose-Canvas $canvas
    Write-ExpectedOcr 'image-columns-01' (
        'FIELD NOTES - PAGE 12' + "`n" +
        'LEFT' + "`n" + $columnsText.left + "`n" +
        'RIGHT' + "`n" + $columnsText.right)
}

# ------------------------------------------------------------- screenshots --

$screenshotLarge = @{
    eyebrow = 'THE QUIET WEB'
    headline = 'A slower corner of the internet'
    body = 'Fewer banners, more reading. We hand-pick one essay a week and send it on Friday.'
    button = 'Join the list'
    footer = 'Already subscribed? Sign in.'
}

function Write-ScreenshotLargeText {
    $W = 1280; $H = 800
    $canvas = New-Canvas $W $H (C 251 250 246)
    $g = $canvas[1]
    Draw-WindowChrome $g $W 'thequietweb.example/'

    $g.DrawString(
        $screenshotLarge.eyebrow,
        (New-Font 'Arial' 18 ([System.Drawing.FontStyle]::Bold)),
        (Brush (C 160 100 60)),
        120, 150)

    $g.DrawString(
        $screenshotLarge.headline,
        (New-Font 'Georgia' 52),
        (Brush (C 40 40 40)),
        [System.Drawing.RectangleF]::new(120, 200, 900, 200))

    $g.DrawString(
        $screenshotLarge.body,
        (New-Font 'Georgia' 26),
        (Brush (C 90 90 90)),
        [System.Drawing.RectangleF]::new(120, 360, 800, 120))

    $g.FillRectangle((Brush (C 239 120 70)), 120, 500, 180, 56)
    $g.DrawString(
        $screenshotLarge.button,
        (New-Font 'Arial' 20 ([System.Drawing.FontStyle]::Bold)),
        (Brush (C 255 255 255)),
        150, 515)

    $g.DrawString(
        $screenshotLarge.footer,
        (New-Font 'Arial' 14),
        (Brush (C 130 130 128)),
        120, 600)

    Save-Png $canvas[0] (Join-Path $ScrDir 'screenshot-large-text-01.png')
    Dispose-Canvas $canvas
    Write-ExpectedOcr 'screenshot-large-text-01' (
        $screenshotLarge.eyebrow + "`n" +
        $screenshotLarge.headline + "`n" +
        $screenshotLarge.body + "`n" +
        $screenshotLarge.button + "`n" +
        $screenshotLarge.footer)
}

$screenshotSmallRows = @(
    @{ Name = 'annual-report-2025.pdf';   Date = 'Jul 02 14:12'; Size = '4.2 MB'; Kind = 'PDF' },
    @{ Name = 'brand-guidelines.pdf';     Date = 'Jun 18 09:40'; Size = '8.7 MB'; Kind = 'PDF' },
    @{ Name = 'field-notes-spring.txt';   Date = 'May 03 18:05'; Size = '12 KB';  Kind = 'Text' },
    @{ Name = 'logo-mark.png';            Date = 'Apr 27 11:22'; Size = '340 KB'; Kind = 'Image' },
    @{ Name = 'meeting-notes.md';         Date = 'Apr 12 16:48'; Size = '6 KB';   Kind = 'Markdown' },
    @{ Name = 'onboarding-flow.pdf';      Date = 'Mar 30 10:02'; Size = '1.9 MB'; Kind = 'PDF' },
    @{ Name = 'photo-batch-01.zip';       Date = 'Mar 21 13:33'; Size = '48 MB';  Kind = 'Archive' },
    @{ Name = 'readme.txt';               Date = 'Mar 02 08:17'; Size = '2 KB';   Kind = 'Text' },
    @{ Name = 'sketchbook-scan.jpg';      Date = 'Feb 14 15:29'; Size = '5.1 MB'; Kind = 'Image' },
    @{ Name = 'timeline-draft.xlsx';      Date = 'Feb 01 09:58'; Size = '22 KB';  Kind = 'Spreadsheet' }
)

function Write-ScreenshotSmallText {
    $W = 1280; $H = 800
    $canvas = New-Canvas $W $H (C 255 255 255)
    $g = $canvas[1]
    Draw-WindowChrome $g $W 'files.example/home/archive'

    $headers = @('Name', 'Modified', 'Size', 'Kind')
    $fontH = New-Font 'Segoe UI' 16 ([System.Drawing.FontStyle]::Bold)
    $fontR = New-Font 'Segoe UI' 14
    $colX = @(120, 560, 780, 900)
    $ink = Brush (C 50 50 50)
    $muted = Brush (C 110 110 108)

    for ($i = 0; $i -lt 4; $i++) {
        $g.DrawString($headers[$i], $fontH, $muted, $colX[$i], 70)
    }

    $y = 120
    $rowIndex = 0
    foreach ($row in $screenshotSmallRows) {
        if ($rowIndex % 2 -eq 0) {
            $g.FillRectangle((Brush (C 247 247 245)), 60, $y, 1160, 46)
        }
        $g.DrawString($row.Name, $fontR, $ink, $colX[0], $y + 12)
        $g.DrawString($row.Date, $fontR, $muted, $colX[1], $y + 12)
        $g.DrawString($row.Size, $fontR, $muted, $colX[2], $y + 12)
        $g.DrawString($row.Kind, $fontR, $muted, $colX[3], $y + 12)
        $y += 46
        $rowIndex++
    }

    Save-Png $canvas[0] (Join-Path $ScrDir 'screenshot-small-text-01.png')
    Dispose-Canvas $canvas

    $expected = 'Name Modified Size Kind' + "`n"
    foreach ($row in $screenshotSmallRows) {
        $expected += ($row.Name + ' ' + $row.Date + ' ' + $row.Size + ' ' + $row.Kind) + "`n"
    }
    Write-ExpectedOcr 'screenshot-small-text-01' $expected
}

# ------------------------------------------------------------- helpers ----

function Write-ExpectedOcr {
    param([string]$Id, [string]$Text)
    [System.IO.File]::WriteAllText(
        (Join-Path $OcrExpDir ($Id + '.txt')),
        $Text,
        [System.Text.UTF8Encoding]::new($false))
}

# ------------------------------------------------------------- pdfs ----

function New-Pdf {
    param(
        [string]$Path,
        [string]$PageStream,
        [byte[]]$Jpeg = $null,
        [int]$ImgW = 0,
        [int]$ImgH = 0
    )

    $ascii = [System.Text.Encoding]::ASCII
    $objects = [System.Collections.Generic.List[byte[]]]::new()

    $pageResources = '/Resources << /Font << /F1 5 0 R >>'
    if ($null -ne $Jpeg) { $pageResources += ' /XObject << /Im0 6 0 R >>' }
    $pageResources += ' >>'

    $objects.Add($ascii.GetBytes('<< /Type /Catalog /Pages 2 0 R >>'))
    $objects.Add($ascii.GetBytes('<< /Type /Pages /Kids [3 0 R] /Count 1 >>'))
    $objects.Add($ascii.GetBytes("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] $pageResources /Contents 4 0 R >>"))

    $streamText = '<< /Length ' + $ascii.GetByteCount($PageStream) + " >>`nstream`n" + $PageStream + "endstream"
    $objects.Add($ascii.GetBytes($streamText))
    $objects.Add($ascii.GetBytes('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>'))

    if ($null -ne $Jpeg) {
        $imgHead = '<< /Type /XObject /Subtype /Image /Width ' + $ImgW + ' /Height ' + $ImgH +
            ' /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ' + $Jpeg.Length + " >>`nstream`n"
        $imgBytes = [System.Collections.Generic.List[byte]]::new()
        $imgBytes.AddRange($ascii.GetBytes($imgHead))
        $imgBytes.AddRange($Jpeg)
        $imgBytes.AddRange($ascii.GetBytes("endstream"))
        $objects.Add($imgBytes.ToArray())
    }

    $count = $objects.Count + 1
    $out = [System.Collections.Generic.List[byte]]::new()
    $out.AddRange($ascii.GetBytes("%PDF-1.4`n"))

    $offsets = [System.Collections.Generic.List[int]]::new()
    for ($i = 0; $i -lt $objects.Count; $i++) {
        $offsets.Add($out.Count)
        $out.AddRange($ascii.GetBytes(($i + 1).ToString() + ' 0 obj' + "`n"))
        $out.AddRange($objects[$i])
        $out.AddRange($ascii.GetBytes("`nendobj`n"))
    }

    $xrefOffset = $out.Count
    $out.AddRange($ascii.GetBytes("xref`n0 $count`n"))
    $out.AddRange($ascii.GetBytes('0000000000 65535 f' + "`n"))
    for ($i = 0; $i -lt $offsets.Count; $i++) {
        $out.AddRange($ascii.GetBytes($offsets[$i].ToString('D10') + ' 00000 n' + "`n"))
    }
    $out.AddRange($ascii.GetBytes("trailer`n<< /Size $count /Root 1 0 R >>`nstartxref`n$xrefOffset`n%%EOF"))

    [System.IO.File]::WriteAllBytes($Path, $out.ToArray())
}

function Escape-PdfText {
    param([string]$Text)
    $Text.Replace('\', '\\').Replace('(', '\(').Replace(')', '\)')
}

function Pdf-Line {
    param([string]$Text, [single]$Size, [single]$X, [single]$Y, [string]$Font = 'F1')
    'BT /' + $Font + ' ' + $Size.ToString('F0') + ' Tf 1 0 0 1 ' + $X.ToString('F0') + ' ' + $Y.ToString('F0') + ' Tm (' + (Escape-PdfText $Text) + ') Tj ET'
}

function Write-NativeTextPdf {
    $lines = @()
    $lines += Pdf-Line 'A Field Guide to Local First Software' 22 72 730
    $lines += Pdf-Line 'by the mymind benchmark team' 12 72 706
    $lines += ''
    $lines += Pdf-Line '1. Introduction' 16 72 660
    $lines += Pdf-Line 'Local first means your data lives on your device and works' 12 72 640
    $lines += Pdf-Line 'without a connection. Your library is a database and a folder' 12 72 626
    $lines += Pdf-Line 'of files, not a remote account.' 12 72 612
    $lines += ''
    $lines += Pdf-Line '2. The Capture Rule' 16 72 576
    $lines += Pdf-Line 'Saving must be faster than deciding where something belongs.' 12 72 556
    $lines += Pdf-Line 'If a note takes three taps to make, people will not take notes.' 12 72 542
    $lines += ''
    $lines += Pdf-Line '3. The Retrieval Promise' 16 72 506
    $lines += Pdf-Line 'Search is the primary organizing system. Tags and folders are' 12 72 486
    $lines += Pdf-Line 'optional furniture; the index is the floor plan.' 12 72 472
    $lines += ''
    $lines += Pdf-Line '4. Closing' 16 72 436
    $lines += Pdf-Line 'Keep it quiet, keep it local, and let the index do the work.' 12 72 416
    $lines += Pdf-Line 'End of field guide.' 12 72 402

    New-Pdf (Join-Path $PdfDir 'pdf-native-text-01.pdf') (($lines -join "`n") + "`n")
}

$memorandum = @(
    'MEMORANDUM',
    '',
    'TO:        All Hands',
    'FROM:      Operations',
    'DATE:      March 12, 2026',
    'RE:        Quarterly Inventory and the Persistent Ghosts of Spring',
    '',
    'Dear team,',
    '',
    'The warehouse count finished at 14,207 units, which is 112 more',
    'than the system predicted. The discrepancy traces back to the west',
    'rack, where boxes are stacked by hand and the barcode scanner gives',
    'up on dust. We will rescan that rack on Friday and credit the',
    'difference.',
    '',
    'The second item is more unusual. Two crates labeled "sprouted',
    'bulbs" have started to germinate in the loading bay. No one ordered',
    'bulbs. No one remembers ordering bulbs. The crates arrived with a',
    'packing slip that reads only: "You know what these are for."',
    '',
    'We are scheduling a meeting to discuss the bulbs.',
    '',
    'Signed,',
    'Operations'
)

function Write-ScannedPdf {
    # Render a typewriter-style page, save as JPEG, embed as a scanned PDF.
    $W = 1240; $H = 1754
    $canvas = New-Canvas $W $H (C 250 247 240)
    $g = $canvas[1]

    $font = New-Font 'Courier New' 34
    $ink = Brush (C 30 40 50)
    $y = 180
    foreach ($lineText in $memorandum) {
        $g.DrawString($lineText, $font, $ink, 150, $y)
        $y += 62
    }

    $tmpJpeg = Join-Path $PdfDir '_scanned-page-01.tmp.jpg'
    Save-Jpeg $canvas[0] $tmpJpeg 85
    $jpegBytes = [System.IO.File]::ReadAllBytes($tmpJpeg)
    Dispose-Canvas $canvas

    New-Pdf (Join-Path $PdfDir 'pdf-scanned-01.pdf') ("q`n612 0 0 792 0 0 cm`n/Im0 Do`nQ`n") $jpegBytes 1240 1754
    Remove-Item $tmpJpeg -Force

    Write-ExpectedOcr 'pdf-scanned-01' ($memorandum -join "`n")
}

function Wrap-PdfColumns {
    param([string]$Text)
    $lines = @()
    $current = ''
    foreach ($w in ($Text -split ' ')) {
        if (($current + ' ' + $w).Trim().Length -le 55) {
            $current = ($current + ' ' + $w).Trim()
        } else {
            $lines += $current
            $current = $w
        }
    }
    if ($current) { $lines += $current }
    return $lines
}

function Write-MulticolumnPdf {
    $left = 'The first column opens with a question that has no tidy answer, so it keeps going instead. Newspapers were built this way: two thoughts side by side, each pretending the other does not exist. Column geometry is a social contract between the author and the margin.'
    $right = 'The second column is shorter on purpose. It prefers a rumor to a paragraph and a margin note to a chapter. A magazine trusts its readers the way a deck trusts its railings: enough to lean, never enough to leave.'

    $lines = @()
    $lines += Pdf-Line 'Field Notes - Two Columns' 20 72 750
    $lines += ''

    $linesL = Wrap-PdfColumns $left
    $linesR = Wrap-PdfColumns $right

    $max = [Math]::Max($linesL.Count, $linesR.Count)
    for ($i = 0; $i -lt $max; $i++) {
        $y = 700 - $i * 18
        if ($i -lt $linesL.Count) { $lines += Pdf-Line $linesL[$i] 12 72 $y }
        if ($i -lt $linesR.Count) { $lines += Pdf-Line $linesR[$i] 12 330 $y }
    }

    New-Pdf (Join-Path $PdfDir 'pdf-multicolumn-01.pdf') (($lines -join "`n") + "`n")
}

function Write-TablesPdf {
    $rows = @(
        @{ Quarter = 'Q1'; Revenue = '28,400'; Expenses = '19,700'; Notes = 'flat' },
        @{ Quarter = 'Q2'; Revenue = '31,150'; Expenses = '21,300'; Notes = 'grew' },
        @{ Quarter = 'Q3'; Revenue = '29,900'; Expenses = '22,100'; Notes = 'dipped' },
        @{ Quarter = 'Q4'; Revenue = '34,600'; Expenses = '23,400'; Notes = 'best' }
    )

    $lines = @()
    $lines += Pdf-Line 'Quarterly Operating Summary' 20 72 750
    $lines += Pdf-Line 'All figures in local currency, unaudited.' 10 72 730
    $lines += ''

    $left = 90; $w1 = 110; $w2 = 140; $w3 = 140; $w4 = 120
    $totalW = $w1 + $w2 + $w3 + $w4
    $rowY = 690; $rowH = 30
    $headers = @(@{ T = 'Quarter'; W = $w1 }, @{ T = 'Revenue'; W = $w2 }, @{ T = 'Expenses'; W = $w3 }, @{ T = 'Notes'; W = $w4 })

    $lines += '0.86 0.86 0.84 rg'
    $lines += ('{0} {1} {2} {3} re f' -f $left, $rowY, $totalW, $rowH)
    $lines += '0 0 0 rg'
    $x = $left
    foreach ($h in $headers) {
        $lines += Pdf-Line $h.T 12 ($x + 8) ($rowY + 8)
        $x += $h.W
    }

    foreach ($r in $rows) {
        $rowY -= $rowH
        $lines += '0.93 0.93 0.92 rg'
        $lines += ('{0} {1} {2} {3} re f' -f $left, $rowY, $totalW, $rowH)
        $lines += '0 0 0 rg'
        $x = $left
        $cells = @($r.Quarter, $r.Revenue, $r.Expenses, $r.Notes)
        for ($c = 0; $c -lt $cells.Count; $c++) {
            $lines += Pdf-Line $cells[$c] 12 ($x + 8) ($rowY + 8)
            $x += $headers[$c].W
        }
    }

    $lines += '0.7 0.7 0.7 RG'
    $topY = 690 + $rowH
    $bottomY = $rowY
    $x = $left
    foreach ($h in $headers) {
        $lines += ('{0} {1} {2} {3} re S' -f $x, $bottomY, 1, ($topY - $bottomY))
        $x += $h.W
    }
    $lines += ('{0} {1} {2} {3} re S' -f $left, $bottomY, $totalW, ($topY - $bottomY))
    for ($yy = $topY; $yy -ge $bottomY; $yy -= $rowH) {
        $lines += ('{0} {1} {2} {3} re S' -f $left, $yy, $totalW, 1)
    }

    New-Pdf (Join-Path $PdfDir 'pdf-tables-01.pdf') (($lines -join "`n") + "`n")
}

function Write-PdfImagesCaptions {
    # A report page that embeds a small image plus a caption.
    $W = 640; $H = 360
    $canvas = New-Canvas $W $H (C 236 218 190)
    $g = $canvas[1]
    $g.FillEllipse((Brush (C 190 120 70)), 80, 60, 200, 200)
    $g.FillPolygon((Brush (C 96 140 100)), @(
        [System.Drawing.PointF]::new(360, 300),
        [System.Drawing.PointF]::new(420, 120),
        [System.Drawing.PointF]::new(480, 300)))
    $g.FillPolygon((Brush (C 96 140 100)), @(
        [System.Drawing.PointF]::new(430, 300),
        [System.Drawing.PointF]::new(520, 160),
        [System.Drawing.PointF]::new(560, 300)))
    $tmpJpeg = Join-Path $PdfDir '_figure-01.tmp.jpg'
    Save-Jpeg $canvas[0] $tmpJpeg 85
    $jpegBytes = [System.IO.File]::ReadAllBytes($tmpJpeg)
    Dispose-Canvas $canvas

    $lines = @()
    $lines += Pdf-Line 'Site Survey Report - West Meadow' 18 72 760
    $lines += Pdf-Line 'Figure 1 shows the new tree line planted after the frost.' 12 72 740
    $lines += Pdf-Line 'The oaks are spaced nine meters apart to leave room for' 12 72 726
    $lines += Pdf-Line 'the wildflower corridor.' 12 72 712
    $lines += ''
    $lines += 'q 460 0 0 260 76 300 cm /Im0 Do Q'
    $lines += ''
    $lines += Pdf-Line 'Figure 1: New tree line, west meadow.' 10 72 268

    New-Pdf (Join-Path $PdfDir 'pdf-images-captions-01.pdf') (($lines -join "`n") + "`n") $jpegBytes 640 360
    Remove-Item $tmpJpeg -Force
}

function Write-BadMetadataPdf {
    # Deliberately minimal: no title, no author, no document information dict.
    $lines = @()
    $lines += Pdf-Line 'untitled fragment' 16 72 720
    $lines += Pdf-Line 'This page ships with no title, no author, and no metadata.' 12 72 700
    $lines += Pdf-Line 'Extraction should still surface the text above.' 12 72 686
    New-Pdf (Join-Path $PdfDir 'pdf-bad-metadata-01.pdf') (($lines -join "`n") + "`n")
}

# ------------------------------------------------------------- run ----

Write-Photo
Write-DesignRef
Write-SimilarA
Write-SimilarB
Write-Distractor
Write-LowRes
Write-Meme
Write-Handwriting
Write-RotatedText
Write-Columns
Write-ScreenshotLargeText
Write-ScreenshotSmallText
Write-NativeTextPdf
Write-ScannedPdf
Write-MulticolumnPdf
Write-TablesPdf
Write-PdfImagesCaptions
Write-BadMetadataPdf

Write-Host 'Corpus generation complete.'
