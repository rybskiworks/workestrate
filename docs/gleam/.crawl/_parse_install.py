import re, sys
raw = open('/home/node/Development/ai-workbench/docs/gleam/.crawl/_raw-install.html', encoding='utf-8').read()
raw = re.sub(r'<script[^>]*>.*?</script>', '', raw, flags=re.DOTALL|re.IGNORECASE)
raw = re.sub(r'<style[^>]*>.*?</style>', '', raw, flags=re.DOTALL|re.IGNORECASE)
from html.parser import HTMLParser
class P(HTMLParser):
    def __init__(self):
        super().__init__()
        self.out=[]; self.skip=0; self.links=[]
    def handle_starttag(self,tag,attrs):
        d=dict(attrs)
        if tag in ('nav','header','footer','aside'): self.skip+=1
        if tag=='a' and 'href' in d: self.links.append(d['href'])
        if tag in ('p','li','h1','h2','h3','h4','pre','code','div','section','br','option'): self.out.append('\n')
    def handle_endtag(self,tag):
        if tag in ('nav','header','footer','aside') and self.skip>0: self.skip-=1
        if tag in ('p','li','h1','h2','h3','h4','pre'): self.out.append('\n')
    def handle_data(self,data):
        if self.skip==0: self.out.append(data)
p=P(); p.feed(raw)
text=''.join(p.out)
text=re.sub(r'[ \t]+',' ',text)
text=re.sub(r'\n{3,}','\n\n',text)
print('===TEXT==='); print(text.strip())
print('===LINKS===')
for l in p.links: print(l)
