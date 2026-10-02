# Renders the demo desktop states with ImageMagick: wallpaper + a mock Settings window.
import subprocess
W,H=1920,1080
WX,WY,WW,WH=320,150,1280,780
SIDEBAR=["Display","Sound","Network","Appearance","Power","Privacy"]
ROWS=[("Night light","Warmer colours after sunset"),("Do not disturb","Silence notifications"),("Auto brightness","Adapt to ambient light"),("Large text","Scale interface text by 125%")]
FONT="Open-Sans"; FONTB="Open-Sans-SemiBold"
def toggle(x,y,on):
    bg="#4c9dff" if on else "#3a3f4b"; kx=x+26 if on else x+4
    return [f"fill {bg} roundrectangle {x},{y} {x+48},{y+26} 13,13", f"fill #f4f6fa circle {kx+9},{y+13} {kx+9},{y+3}"]
def render(name,query="",filtered=False,night_on=False):
    d=[]
    d.append(f"fill #1c1f26 roundrectangle {WX},{WY} {WX+WW},{WY+WH} 16,16")
    d.append(f"fill #22262f roundrectangle {WX},{WY} {WX+300},{WY+WH} 16,16")
    d.append(f"fill #22262f rectangle {WX+284},{WY} {WX+300},{WY+WH}")
    for i,(cx) in enumerate(["#ff5f57","#febc2e","#28c840"]): d.append(f"fill {cx} circle {WX+28+i*22},{WY+28} {WX+34+i*22},{WY+28}")
    for i,item in enumerate(SIDEBAR):
        y=WY+90+i*52
        if item=="Display": d.append(f"fill #2f3542 roundrectangle {WX+16},{y-10} {WX+284},{y+32} 10,10")
        d.append(f"fill #6b7385 roundrectangle {WX+34},{y} {WX+56},{y+22} 6,6")
    # search field
    sx,sy=WX+340,WY+40
    d.append(f"fill #2a2f3a stroke #3d4452 stroke-width 1 roundrectangle {sx},{sy} {WX+WW-40},{sy+48} 12,12")
    d.append("stroke none")
    rows=[r for r in ROWS if not filtered or r[0].lower().startswith(query.lower())]
    for i,(t,sub) in enumerate(rows):
        y=WY+130+i*96
        d.append(f"fill #232833 roundrectangle {WX+340},{y} {WX+WW-40},{y+80} 12,12")
        d+=toggle(WX+WW-110,y+27,night_on and t=="Night light")
    cmd=["magick","-size",f"{W}x{H}","gradient:#0d1b2a-#1b263b","(","-size",f"{W}x{H}","radial-gradient:#3a2d6b-none",")","-compose","screen","-composite",
         "(","+clone","-fill","black","-colorize","100","-fill","white","-draw",f"roundrectangle {WX},{WY+18} {WX+WW},{WY+WH+18} 16,16","-blur","0x28",")",
         "-compose","multiply","-composite"]
    cmd=["magick","-size",f"{W}x{H}","gradient:#0e1626-#1c1433"]
    cmd+=["-fill","none"]
    cmd+=["(","-size",f"{W}x{H}","xc:none","-fill","#00000080","-draw",f"roundrectangle {WX},{WY+16} {WX+WW},{WY+WH+16} 16,16","-blur","0x24",")","-composite"]
    for prim in d: cmd+=["-draw",prim]
    cmd+=["-font",FONTB,"-fill","#e8ebf2","-pointsize","15","-draw",f"text {WX+120},{WY+33} 'Settings'"]
    for i,item in enumerate(SIDEBAR):
        y=WY+90+i*52
        cmd+=["-font",FONT if item!="Display" else FONTB,"-fill","#d7dbe4" if item=="Display" else "#9aa3b5","-pointsize","17","-draw",f"text {WX+70},{y+17} '{item}'"]
    cmd+=["-font",FONT,"-pointsize","17","-fill","#e8ebf2" if query else "#6f7789","-draw",f"text {sx+48},{sy+31} '{query or 'Search settings'}'"]
    cmd+=["-fill","none","-stroke","#8a93a6","-strokewidth","2","-draw",f"circle {sx+24},{sy+22} {sx+30},{sy+22}","-draw",f"line {sx+29},{sy+27} {sx+34},{sy+32}","-stroke","none"]
    for i,(t,sub) in enumerate(rows):
        y=WY+130+i*96
        cmd+=["-font",FONTB,"-pointsize","18","-fill","#e8ebf2","-draw",f"text {WX+370},{y+35} '{t}'"]
        cmd+=["-font",FONT,"-pointsize","15","-fill","#8a93a6","-draw",f"text {WX+370},{y+60} '{sub}'"]
    if night_on:
        cmd+=["(","-size",f"{W}x{H}","xc:#ff9a3c","-alpha","set","-channel","A","-evaluate","set","9%","+channel",")","-composite"]
    cmd+=[f"/demo/{name}.png"]
    subprocess.run(cmd,check=True)
render("a")
render("b",query="night",filtered=True)
render("c",query="night",filtered=True,night_on=True)
