# Presentation
AUTHOR=Your Name Here

## Title of slide 1

You can put regular text paragraphs here. Moreover, you can **bold your text** or *italicize it* very
easily. Alternatively, you can use bullet points:

* Item 1
* Item 2
    * Item 2.2
        * Item 2.2.1
    * Item 2.3

You can also write equations as you would in latex

$$
e^{i \pi} - 1 = 0
$$


## Title Slide 2

![You can fullscreen pictures with captions!](./figs/stock_image.jpg)

## Pictures with text are automatically vertically split

When you have a picture with text on the same slide, the area is automatically split.

* Margins can easily be adjusted in .tex file

![Anoter figure caption here!](./figs/pie_chart.jpg)

## Automatic Code Highlighting

```python
import matplotlib.pyplot as plt
def plot_data(x,y, title):
    plt.scatter(x,y)
    plt.title(title)
    plt.savefig("figure.png")

if __name__ == "__main__":
    print("plotting data")
    plot_data(list(range(5)), [20,15,6,9,10])
```

## In multiple languages!

```fortran
! calculate the kinetic energy of a flowfield
subroutine calculate_energy(energy)
    use m_work ! wrk arrays for velocity
    use m_parameters ! dx, dy, dz

    implicit none
    integer:: i, j, k
    real*8 :: energy, u, v, w

    energy = 0

    do i =1,nx
        do j=1,ny
            do k=1,nz
                u = wrk(i,j,k,1)
                v = wrk(i,j,k,2)
                w = wrk(i,j,k,3)

                energy = energy + u**2 + v**2 + w**2
            end do
        end do
    end do

    energy = energy * dx * dy * dz * 0.5

end subroutine calculate_energy
```

