package packing

import (
	"errors"
	"image"
)

//ErrTooSmall packer is too small error
var ErrTooSmall = errors.New("packer is too small")

//Packer pack rectangle
//        B
//   ------------
//   |rect1|    |             A
//   ------------ A          / \
//   |          |           B   .
//   |          |          / \
//   ------------       rect1  .
//
//          B
//  -------------
//  |       |    |           A
//  |    1  |    |          /  \
//  |------------|A       B     C
//  |  2 |       |      /  \   /  \
//  |    |       |    rect1 . D    .
//d |----|       |           / \
//  |    |       |        rect2 .
//  --------------
//       c
//
type Packer struct {
	root *node
}

//New return a new packer with width and height
func New(w, h int) *Packer {

	root := &node{x: 0, y: 0, w: w, h: h, left: new(node), right: new(node)}

	return &Packer{root: root}
}

// Size return packer's size with and height
func (p *Packer) Size() (int, int) {
	return p.root.w, p.root.h
}

//Insert return Rectangle width final postion
func (p *Packer) Insert(rc image.Rectangle) (image.Rectangle, error) {

	w, h := rc.Dx(), rc.Dy()
	nd := p.root.insert(&node{w: w, h: h}, true)

	if nd == nil {
		return image.Rectangle{}, ErrTooSmall
	}

	rect := image.Rect(nd.x, nd.y, nd.x+nd.w, nd.y+nd.h)

	return rect, nil

}

//Grow packer
//           A
//         /   \
//        B     .
//       /  \
// old root  .
//
func (p *Packer) Grow(width, height int) {
	nd := p.root
	w, h := p.root.w, p.root.h
	p.root = &node{x: 0, y: 0, w: w + width, h: h + height}

	p.root.insert(nd, false)

}

type node struct {
	left, right *node
	x, y, w, h  int
	split       bool
	isEnd       bool
}

func (nd *node) insert(rect *node, isEnd bool) *node {

	if nd.split {
		n := nd.left.insert(rect, isEnd)

		if n != nil {
			return n
		}

		return nd.right.insert(rect, isEnd)

	}

	if nd.isEnd {
		return nil
	}

	if nd.w < rect.w || nd.h < rect.h {
		return nil
	}

	nd.split = true

	if nd.w == rect.w && nd.h == rect.h {

		rect.x = nd.x
		rect.y = nd.y
		nd.isEnd = true
		nd.left = rect
		nd.right = &node{w: 0, h: 0}

		return rect
	}

	dw := nd.w - rect.w
	dh := nd.h - rect.h

	// insert node is down's sibling, they are left's left and right
	var left, right, down *node

	if dw > dh {
		left = &node{x: nd.x, y: nd.y, w: nd.w, h: nd.h - rect.h, split: true}

		down = &node{x: nd.x, y: nd.y + rect.h, w: nd.w - rect.w, h: nd.h - rect.h}
		right = &node{x: nd.x + rect.w, y: nd.y, w: nd.w - rect.w, h: nd.h}

	} else {
		left = &node{x: nd.x, y: nd.y, w: rect.w, h: nd.h, split: true}

		down = &node{x: nd.x, y: nd.y + rect.h, w: nd.w, h: nd.h - rect.h}
		right = &node{x: nd.x + rect.w, y: nd.y, w: nd.w - rect.w, h: rect.h}

	}
	rect.x = right.x - rect.w
	rect.y = down.y - rect.h
	rect.isEnd = isEnd

	left.left = rect
	left.right = down
	nd.left = left
	nd.right = right
	return rect

}
