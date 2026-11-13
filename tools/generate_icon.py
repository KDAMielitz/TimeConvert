from PIL import Image, ImageDraw
from pathlib import Path

# Draw at 4x size for antialiased edges, then package standard Windows sizes.
S = 1024
im = Image.new('RGBA', (S, S))
d = ImageDraw.Draw(im)
d.rounded_rectangle((20,20,1004,1004), radius=235, fill=(17,31,48,255))
d.ellipse((190,190,834,834), outline=(51,211,200,255), width=40)
d.arc((300,190,724,834), 90, 270, fill=(51,126,145,255), width=22)
d.arc((300,190,724,834), 270, 90, fill=(51,126,145,255), width=22)
d.line((225,512,799,512), fill=(51,126,145,255), width=20)
d.line((512,512,512,305), fill=(237,250,250,255), width=48)
d.line((512,512,665,598), fill=(237,250,250,255), width=48)
d.ellipse((478,478,546,546), fill=(51,211,200,255))
icon=im.resize((256,256),Image.Resampling.LANCZOS)
out=Path(__file__).resolve().parent.parent/'assets'
out.mkdir(exist_ok=True)
icon.save(out/'icon.png')
icon.save(out/'icon.ico',sizes=[(n,n) for n in (16,24,32,48,64,128,256)])
